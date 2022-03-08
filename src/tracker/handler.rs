use super::task::{Direction, TrackedData, TrackingTask};
use super::tracker::Command;
use crate::persistance::interface::{Db, Persistance};
use crate::shutdown::Shutdown;
use crate::wrap::API;
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

pub struct TaskHandler<P: Persistance, A: API> {
    /// Task that is being handled by TaskHandler.
    task: TrackingTask,
    /// Shared persistance layer to handle task.
    db: Db<P>,
    /// Shared API for handling task.
    api: Arc<A>,
    /// Indicates whether or not server was shutdown.
    shutdown: Shutdown,
    /// State of currently handled task.
    state: Mutex<State>,
    /// Receives Command regarding running task.
    receiver: Receiver<Command>,
}

impl<P, A> TaskHandler<P, A>
where
    P: Persistance,
    A: API,
{
    pub fn new(
        task: TrackingTask,
        db: Db<P>,
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

    /// Quits running task.
    pub async fn quit(&mut self) {
        let mut state = self.state.lock().await;
        *state = State::Quit;
    }

    /// Stops running task.
    pub async fn stop(&mut self) {
        let mut state = self.state.lock().await;
        *state = State::Stopped;
    }

    /// Stars stopped task.
    pub async fn resume(&mut self) {
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
        let mut timer = tokio::time::interval(self.task.get_interval());
        info!("handler starting with: {} task", self.task.get_id());
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!("handler is shutting down");
                    // If a shutdown signal is received, return from `start`.
                    // This will result in the task terminating.
                    return;
                }
                _ = timer.tick() => {
                    info!("tick");
                    let state = self.state.lock().await;
                    match *state{
                        State::Running => {
                            self.handle().await;
                            if let Some(invocations) = self.task.get_invocations() {
                                counter += 1;
                                if counter >= invocations {
                                    break;
                                }
                            }
                        }
                        State::Stopped => {
                            info!("Task {} stopped", self.task.get_name());
                        }
                        State::Created => {
                            info!("Task {} created, waiting for run", self.task.get_name());
                        }
                        State::Quit => {
                            info!("Task {} is quitting", self.task.get_name());
                            return;
                        }
                    };
                }
            }
        }
    }

    /// Performs single handling of task.
    async fn handle(&self) {
        info!("Handling task {}", self.task.get_name());

        let result = self.task.get_data();
        match result {
            Ok(data) => {
                let last_place = self.db.get(&self.task.get_id()).await.unwrap_or(0);
                let data_len = data.len() as u32;
                info!("last_place: {}, data_len: {}", last_place, data_len);

                let result = self
                    .api
                    .write(
                        create_write_vec(self.task.get_direction(), data.clone()),
                        &self.task.get_spreadsheet_id(),
                        &create_range(
                            &last_place, // TODO: calculations are not working properly.
                            &self.task.get_starting_position(),
                            &self.task.get_sheet(),
                            self.task.get_direction(),
                            data_len,
                        ),
                    )
                    .await;
                info!("saving to db");
                if let Err(err) = self
                    .db
                    .save(self.task.get_id(), data_len + last_place)
                    .await
                {
                    info!("save failed");
                    self.task.run_callbacks(Err(err));
                } else {
                    info!("save successful");
                    self.task.run_callbacks(result);
                }
            }
            Err(e) => {
                self.task.run_callbacks(Err(e));
            }
        }
    }
}

// create_write_vec creates a vector of WriteData from a TrackedData.
fn create_write_vec(direction: Direction, data: TrackedData) -> Vec<Vec<String>> {
    let mut write_vec = Vec::new();
    match direction {
        Direction::Vertical => {
            for v in data {
                write_vec.push(vec![v]);
            }
        }
        Direction::Horizontal => {
            let mut row = Vec::new();
            for v in data {
                row.push(v);
            }
            write_vec.push(row);
        }
    }
    write_vec
}

// create_range creates range from a starting position and a direction.
fn create_range(
    offset: &u32, // last previously written place.
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
    let mut range;
    match direction {
        Direction::Vertical => {
            range = format!(
                "{}{}:{}{}",
                character,
                offset + number,
                character,
                offset + number + data_len
            );
        }
        Direction::Horizontal => {
            range = format!(
                "{}{}:{}{}",
                add_str(character, *offset),
                number,
                add_str(character, offset + data_len),
                number,
            )
        }
    }
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
