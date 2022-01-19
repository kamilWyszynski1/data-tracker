use std::env;
extern crate datatracker_rust;

pub use datatracker_rust::wrap::{APIWrapper, API};

#[tokio::test]
async fn test_api_wrapper() {
    let spreadsheet_id = env::var("SPREADSHEET_ID");
    assert_eq!(spreadsheet_id.is_ok(), true);
    let spreadsheet_id = spreadsheet_id.unwrap();
    assert_eq!(spreadsheet_id.is_empty(), false);
    let api = APIWrapper::new_with_init().await;
    let res = api
        .write(
            vec![vec!["123".to_string(), "1232".to_string()]],
            &spreadsheet_id,
            "A1:A2",
        )
        .await;

    assert!(res.is_ok());

    let res = api.get(&spreadsheet_id, "A1:A2").await;
    assert!(res.is_ok());
    if let Ok(result) = res {
        assert_eq!(result, vec!["123".to_string(), "1232".to_string()])
    }
}
