use super::manager::Command;
use super::task::InputData;
use super::task::TrackingTask;
use super::types::Direction;
use super::types::State;
use crate::error::types::{Error, Result};
use crate::lang::lexer::evaluate_data;
use crate::lang::variable::Variable;
use crate::persistance::interface::Db;
use crate::shutdown::Shutdown;
use crate::wrap::API;
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

/// Describes how TaskHandler is being run.
pub enum HandlerKind {
    Ticker,                                // runs periodically.
    Triggered { ch: Receiver<InputData> }, // runs if something triggers handler.
}

/// Handles single TrackingTask.
pub struct TaskHandler<A: API> {
    /// Task that is being handled by TaskHandler.
    task: TrackingTask,
    /// Shared persistance layer to handle task.
    db: Db,
    /// Shared API for handling task.
    api: Arc<A>,
    /// Indicates whether or not server was shutdown.
    shutdown: Shutdown,
    /// State of currently handled task.
    state: Mutex<State>,
    /// Receives Command regarding running task.
    receiver: Receiver<Command>,
    /// Type of TaskHandler.
    kind: HandlerKind,
}

impl<A> TaskHandler<A>
where
    A: API,
{
    /// Creates new TaskHandler of Ticker kind.
    pub fn new_ticker(
        task: TrackingTask,
        db: Db,
        shutdown: Shutdown,
        api: Arc<A>,
        receiver: Receiver<Command>,
    ) -> Self {
        TaskHandler {
            db,
            shutdown,
            api,
            state: Mutex::new(task.status),
            receiver,
            task,
            kind: HandlerKind::Ticker,
        }
    }

    /// Creates new TaskHandler of Triggered kind.
    pub fn new_triggered(
        task: TrackingTask,
        db: Db,
        shutdown: Shutdown,
        api: Arc<A>,
        receiver: Receiver<Command>,
        ch: Receiver<InputData>,
    ) -> Self {
        TaskHandler {
            db,
            shutdown,
            api,
            state: Mutex::new(task.status),
            receiver,
            task,
            kind: HandlerKind::Triggered { ch },
        }
    }
    async fn apply(&mut self, cmd: Command) -> Result<()> {
        self.change_status(State::from_cmd(cmd)).await
    }

    async fn change_status(&mut self, status: State) -> Result<()> {
        let mut state = self.state.lock().await;
        *state = status;

        self.db.update_task_status(self.task.id, status).await
    }

    // async fn run_signal(&self) -> InputData {
    //     match self.kind.clone() {
    //         HandlerKind::Ticker => {
    //             tokio::time::interval(self.task.interval).tick();
    //             self.task.data().await.unwrap()
    //         }
    //         HandlerKind::Triggered { mut ch } => ch.recv().await.unwrap(),
    //     }
    // }

    pub async fn start(&mut self) {
        info!("handler starting with: {} task", self.task.info());

        match &self.kind {
            HandlerKind::Ticker => self.start_ticker().await,
            HandlerKind::Triggered { mut ch } => self.start_triggered(&mut ch).await,
        }
    }

    /// Starts running task. It runs in loop till shutdown is run.
    ///
    /// Tasks are handled with given interval.
    pub async fn start_ticker(&mut self) {
        let mut timer = tokio::time::interval(self.task.interval);

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
                _ = timer.tick() => {
                    info!("tick");

                    let  mut state = self.state.lock().await;
                    match *state{
                        State::Created => {
                            *state = State::Running; // start running task.
                            if let Err(e) = self.db.update_task_status(self.task.id, State::Running).await{
                                error!("failed to change status to Running: {:?}", e);
                                return;
                            }
                        }
                        State::Running => {
                            let input_data = self.task.data().await.unwrap();
                            info!("got from data_fn: {:?}", input_data);
                            self.handle(&input_data).await;
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

    pub async fn start_triggered(&mut self, ch: &mut Receiver<InputData>) {
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
                input_data = ch.recv() => {
                    let input_data = input_data.unwrap();
                    info!("received from channel: {:?}", input_data);

                    let  mut state = self.state.lock().await;
                    match *state{
                        State::Created => {
                            *state = State::Running; // start running task.
                            if let Err(e) = self.db.update_task_status(self.task.id, State::Running).await{
                                error!("failed to change status to Running: {:?}", e);
                                return;
                            }
                        }
                        State::Running => {
                            self.handle(&input_data).await;
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
                info!("last_place: {}, data_len: {}", last_place, data_len);

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

                info!("saving to db");
                if let Err(err) = self.db.save(self.task.id, data_len + last_place).await {
                    info!("save failed");
                    self.task.run_callbacks(Err(err));
                } else {
                    info!("save successful");
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
    use crate::core::manager::Command;
    use crate::core::task::{InputData, TaskInput, TrackingTask};
    use crate::core::types::*;
    use crate::error::types::Result;
    use crate::lang::engine::Definition;
    use crate::lang::lexer::EvalForest;
    use crate::persistance::interface::{Db, MockPersistance};
    use crate::shutdown::Shutdown;
    use crate::wrap::MockAPI;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::broadcast;
    use tokio::sync::mpsc::{channel, Receiver, Sender};

    use uuid::Uuid;

    use super::TaskHandler;

    async fn test_get_data_fn() -> Result<InputData> {
        Ok(InputData::String(String::from("test")))
    }
    fn test_run() {
        let eval_forest = EvalForest::from_definition(&Definition::new(vec![
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
            String::from("DEFINE(var3, EXTRACT(GET(var), use))"),
            String::from("DEFINE(var4, EXTRACT(GET(var), n))"),
        ]));

        let id = Uuid::parse_str("a54a0fb9-25c9-4f73-ad82-0b7f30ca1ab6").unwrap();
        let tt = TrackingTask {
            id,
            name: Some(String::from("name")),
            description: Some(String::from("description")),
            data_fn: Arc::new(Box::new(move || Box::pin(test_get_data_fn()))),
            spreadsheet_id: String::from("spreadsheet_id"),
            starting_position: String::from("starting_position"),
            sheet: String::from("sheet"),
            direction: Direction::Vertical,
            interval: Duration::from_secs(1),
            with_timestamp: true,
            timestamp_position: TimestampPosition::Before,
            invocations: Some(1),
            eval_forest,
            input: TaskInput::default(),
            callbacks: None,
            status: State::Created,
        };

        let mock_api = MockAPI::new();
        let mock_per = MockPersistance::new();
        let db = Db::new(Box::new(mock_per));

        let (shutdown_notify, shutdown) = broadcast::channel(1);
        let sd = Shutdown::new(shutdown);

        let (send, receiver) = channel::<Command>(1);

        let th = TaskHandler::new_ticker(tt, db, sd, Arc::new(mock_api), receiver);
    }
}
