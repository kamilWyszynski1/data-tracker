use crate::wrap::API;
use std::marker::{Send, Sync};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use std::vec::Vec;
use uuid;

// TrackedData is a type wrap for data that is being tracked. It'll be written as string anyway.
type TrackedData = Vec<String>;

// GetDataFn is a type wrap for a function that returns a TrackedData.
type GetDataFn = fn() -> Result<TrackedData, String>;

#[derive(Clone, Debug, Copy)]
// Direction indicates direction of written data.
pub enum Direction {
    Vertical,   // data will be written in columns.
    Horizontal, // data will be written in rows.
}
#[derive(Clone, Debug, Copy, PartialEq)]
// TimestampPosition indicates position of timestamp in the data.
pub enum TimestampPosition {
    None, // timestamp will not be written.
    Before,
    After,
}

// CallbackFn is a type wrap for callback function.
type CallbackFn = fn(Result<(), String>) -> ();

#[derive(Clone)]
// TrackingTask holds information about tracking task.
pub struct TrackingTask {
    id: uuid::Uuid,              // task id.
    name: Option<String>,        // task name.
    description: Option<String>, // task description.

    get_data_fn: GetDataFn,    // function that returns data to be written.
    spreadsheet_id: String,    // spreadsheet where data will be written.
    starting_position: String, // starting position of data in the spreadsheet. A1 notation.
    sheet: String,             // exact sheet of spreadsheet. Default is empty, first sheet.
    direction: Direction,
    interval: Duration,   // interval between data writes.
    with_timestamp: bool, // whether to write timestamp.
    timestamp_position: TimestampPosition,
    invokations: Option<i32>, // number of invokations.

    callbacks: Option<Vec<CallbackFn>>,
}

unsafe impl Send for TrackingTask {}

impl TrackingTask {
    // creates new TrackingTask.
    pub fn new(
        spreadsheet_id: String,
        sheet: String,
        starting_position: String,
        direction: Direction,
        get_data_fn: GetDataFn,
        interval: Duration,
    ) -> TrackingTask {
        assert_ne!(spreadsheet_id, "", "spreadsheet_id cannot be empty");
        assert!(
            starting_position.len() >= 2,
            "starting_position must be at least 2 characters long."
        );
        TrackingTask {
            id: uuid::Uuid::new_v4(),
            name: None,
            description: None,
            get_data_fn,
            spreadsheet_id,
            sheet,
            starting_position,
            direction,
            interval,
            callbacks: None,
            with_timestamp: false,
            timestamp_position: TimestampPosition::None,
            invokations: None,
        }
    }

    // sets task name.
    pub fn with_name(mut self, name: String) -> TrackingTask {
        self.name = Some(name);
        self
    }

    // sets task description.
    pub fn with_description(mut self, description: String) -> TrackingTask {
        self.description = Some(description);
        self
    }

    // adds callback to task.
    pub fn with_callback(mut self, callback_fn: CallbackFn) -> TrackingTask {
        if self.callbacks.is_none() {
            self.callbacks = Some(Vec::new());
        }
        self.callbacks.as_mut().unwrap().push(callback_fn);
        self
    }

    // with_timestmap adds timestamp to data.
    pub fn with_timestmap(
        mut self,
        with_timestamp: bool,
        position: TimestampPosition,
    ) -> TrackingTask {
        self.with_timestamp = with_timestamp;
        assert!(
            position != TimestampPosition::None,
            "Timestamp position cannot be None."
        );
        self.timestamp_position = position;
        self
    }

    // with_invokations sets number of invokations.
    pub fn with_invokations(mut self, invokations: i32) -> TrackingTask {
        self.invokations = Some(invokations);
        self
    }

    // runs task callbacks on result.
    pub fn run_callbacks(&self, result: Result<(), String>) {
        if let Some(callbacks) = &self.callbacks {
            for callback in callbacks {
                callback(result.clone());
            }
        }
    }

    pub fn get_data(&self) -> Result<TrackedData, String> {
        (self.get_data_fn)()
    }

    pub fn get_name(&self) -> &str {
        if let Some(name) = &self.name {
            name
        } else {
            "No name"
        }
    }
}

// Tracker is a wrapper for the Google Sheets API.
// It is used to track various kind of things and keep that data in a Google Sheet.
pub struct Tracker<A: 'static + API + Sync + Send + Clone> {
    api: A,
    tasks: Vec<TrackingTask>,
}

impl<A> Tracker<A>
where
    A: 'static + API + Sync + Send + Clone,
{
    // creates new Tracker.
    pub fn new(api: A) -> Self {
        Tracker {
            api,
            tasks: Vec::new(),
        }
    }

    // adds new task to Tracker.
    pub fn add_task(&mut self, task: TrackingTask) {
        self.tasks.push(task);
    }

    // runs all tasks.
    pub async fn run(&self) {
        let mut joins = Vec::new(); // create vector of JoinHandle, we will join them later.

        for task in &self.tasks {
            println!("Running task {}", task.get_name());
            let task = Arc::new(task.clone());
            let api = Arc::new(self.api.clone());
            joins.push(tokio::task::spawn(
                async move { schedule_task(task, api).await },
            ));
        }

        for join in joins {
            join.await.unwrap();
        }
        println!("All tasks finished.");
    }
}

pub async fn schedule_task<A: 'static + API + Sync + Send + Clone>(
    task: Arc<TrackingTask>,
    api: Arc<A>,
) {
    println!("Starting task {}", task.get_name());

    let mut counter = 0; // invokations counter. Will not be used if invokations is None.
    let mut timer = tokio::time::interval(task.interval);
    loop {
        timer.tick().await;
        handle_task(task.clone(), api.clone()).await;
        if let Some(invokations) = task.invokations {
            counter += 1;
            if counter >= invokations {
                break;
            }
        }
    }
    println!("Task {} finished.", task.get_name());
}

// handles single task.
async fn handle_task<A: 'static + API + Sync + Send + Clone>(task: Arc<TrackingTask>, api: Arc<A>) {
    println!("Handling task {}", task.get_name());

    let result = task.get_data();
    match result {
        Ok(data) => {
            let result = api
                .write(
                    create_write_vec(task.direction, data.clone()),
                    &task.spreadsheet_id,
                    &create_range(&task.starting_position, &task.sheet, task.direction, data),
                )
                .await;
            task.run_callbacks(result);
        }
        Err(e) => {
            task.run_callbacks(Err(e));
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
    starting_position: &str,
    sheet: &str,
    direction: Direction,
    data: TrackedData,
) -> String {
    let character = &starting_position[..1];
    assert!(
        character.len() == 1,
        "Starting position must be a single character."
    );
    let number = starting_position[1..].parse::<usize>().unwrap();
    let mut range = String::from(starting_position);
    match direction {
        Direction::Vertical => {
            range.push_str(&format!(":{}{}", character, number + data.len()));
        }
        Direction::Horizontal => {
            range.push_str(&format!(
                ":{}{}",
                add_str(character, data.len() as u32),
                number
            ));
        }
    }
    if sheet != "" {
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
    use crate::tracker::TrackedData;

    fn test_get_data_fn() -> Result<TrackedData, String> {
        Ok(vec!["test".to_string()])
    }
    #[test]
    fn callback_test() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "speadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_callback(|res: Result<(), String>| {
            assert_eq!(res.is_ok(), true);
        });
        assert!(tt.callbacks.is_some());
        tt.run_callbacks(Ok(()));
    }

    #[test]
    fn task_with_name() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "speadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_name("test".to_string());
        assert!(tt.name.is_some());
        assert_eq!(tt.name.unwrap(), "test")
    }

    #[test]
    fn task_with_description() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "speadsheet_id".to_string(),
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
            std::time::Duration::from_secs(1),
        );
        tt = tt.with_description("test".to_string());
        assert!(tt.description.is_some());
        assert_eq!(tt.description.unwrap(), "test")
    }

    use crate::wrap::API;
    use async_trait::async_trait; // crate for async traits.

    #[derive(Clone)]
    struct TestAPI {
        check: fn(Vec<Vec<String>>, &str, &str),
        fail: bool,
        fail_msg: String,
    }

    #[async_trait]
    impl API for TestAPI {
        async fn write(&self, v: Vec<Vec<String>>, s: &str, r: &str) -> Result<(), String> {
            (self.check)(v, s, r);
            if self.fail {
                return Err(self.fail_msg.clone());
            }
            Ok(())
        }
    }

    #[tokio::test]
    #[timeout(30000)] // 30 sec timeout.
    async fn test_run() {
        fn check_cases(v: Vec<Vec<String>>, s: &str, r: &str) {
            let cases = vec![
                (vec![vec!["test".to_string()]], "spreadsheet1", "A1:A2"),
                (vec![vec!["test".to_string()]], "spreadsheet2", "A1:B1"),
            ];
            println!("{:?} {} {}", cases, s, r);
            for (i, c) in cases.iter().enumerate() {
                if v == c.0 && s == c.1 && r == c.2 {
                    println!("Case {} passed", i);
                    return;
                }
            }
            panic!("failed")
        }

        use crate::tracker::{Direction, Tracker, TrackingTask};
        let mut t = Tracker::new(TestAPI {
            check: check_cases,
            fail: false,
            fail_msg: "".to_string(),
        });
        t.add_task(
            TrackingTask::new(
                "spreadsheet1".to_string(),
                "".to_string(),
                "A1".to_string(),
                Direction::Vertical,
                test_get_data_fn,
                std::time::Duration::from_secs(1),
            )
            .with_name("TEST1".to_string())
            .with_invokations(1),
        );
        t.add_task(
            TrackingTask::new(
                "spreadsheet2".to_string(),
                "".to_string(),
                "A1".to_string(),
                Direction::Horizontal,
                test_get_data_fn,
                std::time::Duration::from_secs(1),
            )
            .with_name("TEST2".to_string())
            .with_invokations(1),
        );

        t.run().await;
    }

    #[tokio::test]
    #[timeout(30000)] // 30 sec timeout.
    async fn test_run_callback() {
        fn check_cases(_: Vec<Vec<String>>, _: &str, _: &str) {}
        fn callback(res: Result<(), String>) {
            assert_eq!(res.is_err(), true);
            match res {
                Err(e) => {
                    assert_eq!(e, "fail".to_string());
                }
                _ => panic!("failed"),
            }
        }

        use crate::tracker::{Direction, Tracker, TrackingTask};
        let mut t = Tracker::new(TestAPI {
            check: check_cases,
            fail: true,
            fail_msg: "fail".to_string(),
        });

        t.add_task(
            TrackingTask::new(
                "spreadsheet1".to_string(),
                "".to_string(),
                "A1".to_string(),
                Direction::Vertical,
                test_get_data_fn,
                std::time::Duration::from_secs(1),
            )
            .with_name("TEST3".to_string())
            .with_callback(callback)
            .with_invokations(1),
        );
        t.run().await;
    }
}
