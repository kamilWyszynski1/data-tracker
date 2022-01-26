use std::env;
extern crate datatracker_rust;

pub use datatracker_rust::wrap::{APIWrapper, API};

#[tokio::test]
#[ignore]
async fn test_api_wrapper() {
    let spreadsheet_id = env::var("SPREADSHEET_ID");
    assert!(spreadsheet_id.is_ok());
    let spreadsheet_id = spreadsheet_id.unwrap();
    assert!(spreadsheet_id.is_empty());
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
