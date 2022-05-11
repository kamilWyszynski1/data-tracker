use super::channels::ChannelsManager;
use super::manager::Command;
use super::task::InputData;
use super::task::TrackingTask;
use super::types::Direction;
use super::types::State;
use crate::core::types::TaskKind;
use crate::error::types::{Error, Result};
use crate::lang::lexer::evaluate_data;
use crate::lang::variable::Variable;
use crate::persistance::interface::Db;
use crate::shutdown::Shutdown;
use crate::wrap::API;
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

/// Handles single TrackingTask.
pub struct TaskHandler<A: API> {
    /// Task that is being handled by TaskHandler.
    pub task: TrackingTask,
    /// Shared persistance layer to handle task.
    db: Db,
    /// Shared API for handling task.
    api: Arc<A>,
    /// Indicates whether or not server was shutdown.
    shutdown: Shutdown,
    /// State of currently handled task.
    // state: Mutex<State>,
    /// Receives Command regarding running task.
    receiver: Receiver<Command>,
    channels_manager: ChannelsManager,
}

impl<A> TaskHandler<A>
where
    A: API,
{
    /// Creates new TaskHandler of Ticker kind.
    pub fn new(
        task: TrackingTask,
        db: Db,
        shutdown: Shutdown,
        api: Arc<A>,
        receiver: Receiver<Command>,
        channels_manager: ChannelsManager,
    ) -> Self {
        TaskHandler {
            db,
            shutdown,
            api,
            receiver,
            task,
            channels_manager,
        }
    }

    async fn apply(&mut self, cmd: Command) -> Result<()> {
        self.change_status(State::from_cmd(cmd)).await
    }

    async fn change_status(&mut self, status: State) -> Result<()> {
        self.task.status = status;
        self.db.update_task_status(self.task.id, status).await
    }

    pub async fn start(&mut self) {
        self.task
            .init_channels(&self.channels_manager, self.shutdown.subscribe())
            .await;

        if self.task.status == State::Created {
            debug!("saving task on receive: {:?}", self.task);
            self.db.save_task(&self.task).await.unwrap();
        }
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!("handler is shutting down");
                    // If a shutdown signal is received, return from `start`.
                    // This will result in the task terminating.
                    return;
                }
                cmd = self.receiver.recv() => {
                    info!("applying {:?} cmd for {} task", cmd, self.task.info());
                    match cmd {
                        None => {
                            info!("receiver has been closed for {} task, returning", self.task.info());
                            return;
                        }
                        Some(command) => {
                            if let Err(err) = self.apply(command).await {
                                error!("failed to apply command to task: {:?}", err)
                            }
                        }
                    }
                }
                id = run_signal(&self.task) => {
                    info!("got data from run_signal: {:?}", id);
                    match self.task.status{
                        State::Created => {
                            if let Err(e) = self.change_status(State::Running).await{
                                error!("failed to change status to Running: {:?}", e);
                                return;
                            };
                            self.handle(&id).await;
                        }
                        State::Running => {
                            self.handle(&id).await;
                        }
                        State::Stopped => {
                            info!("Task {} stopped", self.task.info());
                        }
                        State::Quit => {
                            if let Err(err) = self.db.delete_task(self.task.id).await {
                                error!("failed to delete task: {:?}", err);
                            }
                            info!("Task {} is quitting", self.task.info());
                            return;
                        }
                    };
                }
            }
        }
    }

    /// Performs single handling of task.
    async fn handle(&self, input_data: &InputData) {
        info!("Handling task {}", self.task.info());

        let evaluated = evaluate_data(input_data, &self.task.eval_forest);
        match evaluated {
            Ok(data) => {
                info!("evaluated from engine: {:?}", &data);

                let data = create_write_vec(self.task.direction, data);

                let last_place = self.db.get(&self.task.id).await.unwrap_or(0);
                let data_len = data.len() as u32;
                debug!("last_place: {}, data_len: {}", last_place, data_len);

                let result = self
                    .api
                    .write(
                        data,
                        &self.task.spreadsheet_id,
                        &create_range(
                            last_place, // TODO: calculations are not working properly.
                            &self.task.starting_position,
                            &self.task.sheet,
                            self.task.direction,
                            data_len,
                        ),
                    )
                    .await;

                debug!("saving to db");
                if let Err(err) = self.db.save(self.task.id, data_len + last_place).await {
                    debug!("save failed");
                    self.task.run_callbacks(Err(err));
                } else {
                    debug!("save successful");
                    self.task.run_callbacks(result);
                }
            }
            Err(err) => {
                error!("{:?}", err);
                self.task.run_callbacks(Err(Error::new_internal(
                    String::from("get"),
                    String::from("failed to evaluate"),
                    err.to_string(),
                )));
            }
        }
        if self.task.invocations.is_some() {
            self.task.invocations.map(|i| i - 1);
        }
    }
}

async fn run_signal(task: &TrackingTask) -> InputData {
    assert!(task.kind.is_some());
    let kind = task.kind.as_ref().unwrap();
    match kind {
        TaskKind::Triggered { ch } => ch.lock().await.recv().await.unwrap(),
        TaskKind::Ticker { interval } => {
            let mut timer = tokio::time::interval(*interval);
            timer.tick().await;
            task.data().await.unwrap()
        }
        TaskKind::Clicked { ch } => {
            ch.lock().await.recv().await.unwrap(); // wait for an click/call event.
            task.data().await.unwrap() // return configured data.
        }
    }
}

// create_write_vec creates a vector of WriteData from a TrackedData.
fn create_write_vec(_direction: Direction, data: Variable) -> Vec<Vec<String>> {
    let mut write_vec: Vec<Vec<String>> = Vec::new();
    write_vec.push(vec![format!("{:?}", data)]);
    write_vec
}

fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

// create_range creates range from a starting position and a direction.
fn create_range(
    offset: u32, // last previously written place.
    starting_position: &str,
    sheet: &str,
    direction: Direction,
    data_len: u32,
) -> String {
    let character = &starting_position[..1];
    assert!(
        character.len() == 1,
        "Starting position must be a single character."
    );
    let number = starting_position[1..].parse::<u32>().unwrap();

    let mut range: String = match direction {
        Direction::Vertical => {
            format!(
                "{}{}:{}{}",
                character,
                offset + number,
                character,
                offset + number + data_len
            )
        }
        Direction::Horizontal => {
            format!(
                "{}{}:{}{}",
                add_str(character, offset),
                number,
                add_str(character, offset + data_len),
                number,
            )
        }
    };
    if !sheet.is_empty() {
        range = format!("{}!{}", sheet, range);
    }
    debug!("range: {:?}", range);
    range
}

// add_str increase ASCII code of a character by a number.
fn add_str(s: &str, increment: u32) -> String {
    s.chars()
        .map(|c| std::char::from_u32(c as u32 + increment).unwrap_or(c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::TaskHandler;
    use crate::core::channels::ChannelsManager;
    use crate::core::handler::run_signal;
    use crate::core::manager::Command;
    use crate::core::task::{BoxFnThatReturnsAFuture, InputData, TrackingTask};
    use crate::core::types::{Direction, Hook, State, TaskKind};
    use crate::error::types::Result;
    use crate::persistance::in_memory::InMemoryPersistance;
    use crate::persistance::interface::Db;
    use crate::server::task::TaskKindRequest;
    use crate::shutdown::Shutdown;
    use crate::wrap::TestAPI;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::select;
    use tokio::sync::{broadcast, mpsc, Mutex};

    fn empty_shutdown() -> broadcast::Receiver<()> {
        let (_, receiver) = broadcast::channel(1);
        receiver
    }
    async fn test_get_data_fn() -> Result<InputData> {
        Ok(InputData::String(String::from("test")))
    }

    fn data_fn() -> Option<BoxFnThatReturnsAFuture> {
        Some(Box::new(move || Box::pin(test_get_data_fn())))
    }

    #[tokio::test]
    async fn test_run_signal_ticker() {
        let mut tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            data_fn(),
            TaskKindRequest::Ticker { interval_secs: 1 },
        );
        tt.init_channels(&ChannelsManager::default(), empty_shutdown())
            .await;
        let id = run_signal(&tt).await;
        assert_eq!(id, InputData::String(String::from("test")))
    }

    #[tokio::test]
    async fn test_run_signal_triggered() {
        let (sender, receiver) = mpsc::channel(1);

        let tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            None,
            TaskKindRequest::Ticker { interval_secs: 1 },
        )
        .with_kind(TaskKind::Triggered {
            ch: Arc::new(Mutex::new(receiver)),
        });
        sender
            .send(InputData::Vector(vec![InputData::String(String::from(
                "triggered",
            ))]))
            .await
            .unwrap();
        let id = run_signal(&tt).await;
        assert_eq!(
            id,
            InputData::Vector(vec![InputData::String(String::from("triggered"))])
        )
    }

    #[tokio::test]
    async fn test_run_signal_clicked() {
        let (sender, receiver) = mpsc::channel(1);

        let tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            data_fn(),
            TaskKindRequest::Ticker { interval_secs: 1 },
        )
        .with_kind(TaskKind::Clicked {
            ch: Arc::new(Mutex::new(receiver)),
        });
        sender.send(()).await.unwrap();

        let id = run_signal(&tt).await;
        assert_eq!(id, InputData::String(String::from("test")))
    }

    #[tokio::test]
    async fn test_handler() {
        env_logger::try_init();

        let tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1".to_string(),
            Direction::Vertical,
            data_fn(),
            TaskKindRequest::Ticker { interval_secs: 1 },
        );

        let db = InMemoryPersistance::new();
        let channels_manager = ChannelsManager::default();
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);
        let shutdown = Shutdown::new(shutdown_sender.clone(), shutdown_receiver);
        let (api, mut ch) = TestAPI::new();
        let api = Arc::new(api);
        let (cmd_sender, cmd_receiver) = mpsc::channel(1);
        let mut handler = TaskHandler::new(
            tt,
            Db::new(Box::new(db)),
            shutdown,
            api,
            cmd_receiver,
            channels_manager,
        );

        tokio::task::spawn(async move { handler.start().await });

        loop {
            select! {
                result = ch.recv() => {
                    match result {
                        Some(result) => {
                            assert_eq!(result, vec![vec![String::from(r#"String("test")"#)]]);
                            break;
                        },
                        None => (),
                    }
                }
            }
        }
        drop(shutdown_sender);
        drop(cmd_sender);
    }

    #[tokio::test]
    async fn test_handler_triggered() {
        env_logger::try_init();

        let tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1".to_string(),
            Direction::Vertical,
            data_fn(),
            TaskKindRequest::Triggered(Hook::None),
        );
        let id = tt.id.clone();

        let db = InMemoryPersistance::new();
        let channels_manager = ChannelsManager::default();
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);
        let shutdown = Shutdown::new(shutdown_sender.clone(), shutdown_receiver);
        let (api, mut ch) = TestAPI::new();
        let api = Arc::new(api);
        let (cmd_sender, cmd_receiver) = mpsc::channel(1);
        let mut handler = TaskHandler::new(
            tt,
            Db::new(Box::new(db)),
            shutdown,
            api,
            cmd_receiver,
            channels_manager.clone(),
        );

        tokio::task::spawn(async move { handler.start().await });
        tokio::time::sleep(Duration::from_millis(100)).await;

        let sender = channels_manager
            .triggered_channels
            .lock()
            .await
            .get(&id)
            .unwrap()
            .clone();

        sender
            .send(InputData::Vector(vec![InputData::String(String::from(
                "triggered",
            ))]))
            .await
            .unwrap();

        loop {
            select! {
                result = ch.recv() => {
                    match result {
                        Some(result) => {
                            assert_eq!(result, vec![vec![String::from(r#"Vector([String("triggered")])"#)]]);
                            break;
                        },
                        None => (),
                    }
                }
            }
        }
        drop(shutdown_sender);
        drop(cmd_sender);
    }

    #[tokio::test]
    async fn test_handler_command() {
        env_logger::try_init();

        let tt = TrackingTask::new(
            "spreadsheet_id".to_string(),
            "".to_string(),
            "A1".to_string(),
            Direction::Vertical,
            data_fn(),
            TaskKindRequest::Triggered(Hook::None),
        );
        let id = tt.id;

        let mut db = Db::new(Box::new(InMemoryPersistance::new()));
        let channels_manager = ChannelsManager::default();
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);
        let shutdown = Shutdown::new(shutdown_sender.clone(), shutdown_receiver);
        let (api, _) = TestAPI::new();
        let api = Arc::new(api);
        let (cmd_sender, cmd_receiver) = mpsc::channel(1);

        // TODO: write task to DB on TaskHandler start.
        let mut handler = TaskHandler::new(
            tt,
            db.clone(),
            shutdown,
            api,
            cmd_receiver,
            channels_manager.clone(),
        );

        tokio::task::spawn(async move { handler.start().await });

        cmd_sender.send(Command::Stop).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(db.read_task(id).await.unwrap().status, State::Stopped);

        cmd_sender.send(Command::Resume).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(db.read_task(id).await.unwrap().status, State::Running);

        cmd_sender.send(Command::Delete).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(db.read_task(id).await.unwrap().status, State::Quit);

        let sender = channels_manager
            .triggered_channels
            .lock()
            .await
            .get(&id)
            .unwrap()
            .clone();
        sender
            .send(InputData::String(String::from("test")))
            .await
            .unwrap(); // task should be deleted from db.
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(db.read_task(id).await.is_err());

        drop(shutdown_sender);
        drop(sender);
    }
}
