use crate::wrap::API;
use std::marker::{Send, Sync};
use std::sync::Arc;
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

// CallbackFn is a type wrap for callback function.
type CallbackFn = fn(Result<(), String>) -> ();

#[derive(Clone)]
// Callback is a type wrap for a function that will be called when data is written.
pub struct Callback(CallbackFn);

#[derive(Clone)]
// TrackingTask holds information about tracking task.
pub struct TrackingTask {
    id: uuid::Uuid,              // task id.
    name: Option<String>,        // task name.
    description: Option<String>, // task description.

    get_data_fn: GetDataFn, // function that returns data to be written.
    spreadsheet_id: String, // spreadsheet where data will be written.
    // sheet is a exact sheet of spreadsheet. Default is empty, first sheet.
    sheet: String,
    direction: Direction,
    callbacks: Option<Vec<Callback>>,
}

unsafe impl Send for TrackingTask {}

impl TrackingTask {
    // creates new TrackingTask.
    pub fn new(
        spreadsheet_id: String,
        sheet: String,
        direction: Direction,
        get_data_fn: GetDataFn,
    ) -> TrackingTask {
        TrackingTask {
            id: uuid::Uuid::new_v4(),
            name: None,
            description: None,
            get_data_fn,
            spreadsheet_id,
            sheet,
            direction,
            callbacks: None,
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
        self.callbacks.as_mut().unwrap().push(Callback(callback_fn));
        self
    }

    // runs task callbacks on result.
    pub fn run_callbacks(&self, result: Result<(), String>) {
        if let Some(callbacks) = &self.callbacks {
            for callback in callbacks {
                callback.0(result.clone());
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

impl<A: 'static + API + Sync + Send + Clone> Tracker<A> {
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
        // for task in &self.tasks {
        //     let api = Arc::new(self.api.clone());
        //     let task = Arc::new(task);
        //     tokio::spawn(run_single_task(api, task));
        // }
        // let task = Arc::new(self.tasks.get(0).unwrap().clone());
        // tokio::spawn(async move { println!("{}", task.get_name()) })
        //     .await
        //     .unwrap();
        let mut joins = Vec::new(); // create vector of JoinHandle, we will join them later.

        for task in &self.tasks {
            let task = Arc::new(task.clone());
            let api = Arc::new(self.api.clone());
            joins.push(tokio::spawn(async move {
                let result = task.get_data();
                match result {
                    Ok(data) => {
                        let result = api
                            .as_ref()
                            .write(
                                create_write_vec(task.as_ref().direction, data),
                                &task.spreadsheet_id,
                                &task.sheet,
                            )
                            .await;
                        task.run_callbacks(result);
                    }
                    Err(e) => {
                        task.run_callbacks(Err(e));
                    }
                }
            }));
        }

        for join in joins {
            join.await.unwrap();
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
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
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
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
        );
        tt = tt.with_name("test".to_string());
        assert!(tt.name.is_some());
        assert_eq!(tt.name.unwrap(), "test")
    }

    #[test]
    fn task_with_description() {
        use crate::tracker::{Direction, TrackingTask};
        let mut tt = TrackingTask::new(
            "".to_string(),
            "A1:B1".to_string(),
            Direction::Vertical,
            test_get_data_fn,
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
    async fn test_run() {
        fn check_cases(v: Vec<Vec<String>>, s: &str, r: &str) {
            let cases = vec![
                (vec![vec!["test".to_string()]], "spreadsheet1", "A1:B1"),
                (vec![vec!["test".to_string()]], "spreadsheet2", "C1:D1"),
            ];
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
                "A1:B1".to_string(),
                Direction::Vertical,
                test_get_data_fn,
            )
            .with_name("name test".to_string()),
        );
        t.add_task(
            TrackingTask::new(
                "spreadsheet2".to_string(),
                "C1:D1".to_string(),
                Direction::Vertical,
                test_get_data_fn,
            )
            .with_name("name test2".to_string()),
        );

        t.run().await;
    }

    #[tokio::test]
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
                "A1:B1".to_string(),
                Direction::Vertical,
                test_get_data_fn,
            )
            .with_name("name test".to_string())
            .with_callback(callback),
        );
        t.run().await;
    }
}
