mod tracker;
mod wrap;

extern crate google_sheets4 as sheets4;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2 as oauth2;
#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting up");
    google_sheets_setup().await;
    info!("Done");
}

async fn google_sheets_setup() {
    let hub = wrap::APIWrapper::new_with_init().await;

    info!("calling google sheets");

    let result = hub
        .write(
            vec![vec!["123".to_string(), "1232".to_string()]],
            "12rVPMk3Lv7VouUZBglDd_oRDf6PHU7m6YbfctmFYYlg",
            "A1:B1",
        )
        .await;
    // .spreadsheets()
    // .values_update(req, "12rVPMk3Lv7VouUZBglDd_oRDf6PHU7m6YbfctmFYYlg", "A1:B1")
    // .include_values_in_response(false)
    // .value_input_option("RAW")
    // .doit()
    // .await;

    match result {
        Err(e) => {
            error!("{:?}", e);
        }
        Ok(_) => {}
    }
}
