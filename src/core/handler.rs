use super::direction::Direction;
use super::intype::InputType;
use super::manager::Command;
use super::task::{InputData, TrackingTask};
use crate::lang::engine::Engine;
use crate::lang::lexer::{EvalError, EvalForest};
use crate::lang::variable::Variable;
use crate::persistance::interface::Db;
use crate::shutdown::Shutdown;
use crate::wrap::API;
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

/// State is state of currently handled task.
enum State {
    Created,
    Running,
    Stopped,
    Quit,
}

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
}

impl<A> TaskHandler<A>
where
    A: API,
{
    pub fn new(
        task: TrackingTask,
        db: Db,
        shutdown: Shutdown,
        api: Arc<A>,
        receiver: Receiver<Command>,
    ) -> Self {
        TaskHandler {
            task,
            db,
            shutdown,
            api,
            state: Mutex::new(State::Created),
            receiver,
        }
    }

    async fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Stop => self.stop().await,
            Command::Delete => self.quit().await,
            Command::Resume => self.resume().await,
        }
    }

    /// Quits running task.
    async fn quit(&mut self) {
        let mut state = self.state.lock().await;
        *state = State::Quit;
    }

    /// Stops running task.
    async fn stop(&mut self) {
        let mut state = self.state.lock().await;
        *state = State::Stopped;
    }

    /// Stars stopped task.
    async fn resume(&mut self) {
        let mut state = self.state.lock().await;
        *state = State::Running;
    }

    /// Starts running task. It runs in loop till shutdown is run.
    ///
    /// Each task is handled per given interval.
    pub async fn start(&mut self) {
        {
            let mut state = self.state.lock().await;
            *state = State::Running;
            // lock dropped here.
        }
        let mut counter = 0; // invocations counter. Will not be used if invocations is None.
        let mut timer = tokio::time::interval(self.task.interval());
        info!("handler starting with: {} task", self.task.info());
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
                            info!("receiver has been closed to {} task, returning", self.task.info());
                            return;
                        }
                        Some(command) => {
                            self.apply(command).await;
                        }
                    }
                }
                _ = timer.tick() => {
                    info!("tick");
                    let state = self.state.lock().await;
                    match *state{
                        State::Running => {
                            self.handle().await;
                            if let Some(invocations) = self.task.invocations() {
                                counter += 1;
                                if counter >= invocations {
                                    break;
                                }
                            }
                        }
                        State::Stopped => {
                            info!("Task {} stopped", self.task.info());
                        }
                        State::Created => {
                            info!("Task {} created, waiting for run", self.task.info());
                        }
                        State::Quit => {
                            info!("Task {} is quitting", self.task.info());
                            return;
                        }
                    };
                }
            }
        }
    }

    /// Performs single handling of task.
    async fn handle(&self) {
        info!("Handling task {}", self.task.info());

        let result = self.task.data().await.unwrap();
        info!("got from data_fn: {:?}", result);

        let evaluated = evaluate_data(result, &self.task.eval_forest);
        info!("evaluated from engine: {:?}", &evaluated);

        match evaluated {
            Ok(data) => {
                let data = create_write_vec(self.task.direction(), data);

                let last_place = self.db.get(&self.task.id()).await.unwrap_or(0);
                let data_len = data.len() as u32;
                info!("last_place: {}, data_len: {}", last_place, data_len);

                let result = self
                    .api
                    .write(
                        data,
                        &self.task.spreadsheet_id(),
                        &create_range(
                            last_place, // TODO: calculations are not working properly.
                            &self.task.starting_position(),
                            &self.task.sheet(),
                            self.task.direction(),
                            data_len,
                        ),
                    )
                    .await;
                info!("saving to db");
                if let Err(err) = self.db.save(self.task.id(), data_len + last_place).await {
                    info!("save failed");
                    self.task.run_callbacks(Err(err));
                } else {
                    info!("save successful");
                    self.task.run_callbacks(result);
                }
            }
            Err(e) => {
                error!("{:?}", e);
                self.task.run_callbacks(Err("failed to evaluate"));
            }
        }
    }
}

/// Function creates new engine and calls fire method for given Definition.
fn evaluate_data(data: InputData, ef: &EvalForest) -> Result<Variable, EvalError> {
    let mut e = Engine::new(Variable::from_input_data(&data));
    e.fire(ef)?;
    Ok(e.get(String::from("OUT"))
        .ok_or(EvalError::Internal {
            operation: String::from("evaluate_data"),
            msg: String::from("There is not OUT variable!!!"),
        })?
        .clone())
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
