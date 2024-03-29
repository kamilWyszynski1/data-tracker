use crate::error::types::Error as IError;
use crate::error::types::Result;
use async_trait::async_trait; // crate for async traits.
use hyper::{body, Body, Response};
use mockall::*;
use sheets4::api::ValueRange;
use sheets4::{Error, Sheets};
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[async_trait]
#[automock]
// API is a wrapper for the Google Sheets API.
pub trait API {
    async fn write(&self, values: Vec<Vec<String>>, sheet_id: &str, range: &str) -> Result<()>;
}

#[derive(Clone)]
// APIWrapper is a implementation of API trait for the Google Sheets API.
pub struct APIWrapper {
    client: sheets4::Sheets,
}

#[async_trait]
impl API for APIWrapper {
    // writes data to a sheet.
    async fn write(&self, values: Vec<Vec<String>>, sheet_id: &str, range: &str) -> Result<()> {
        let req = ValueRange {
            values: Some(values),
            ..Default::default()
        };

        info!("writing {}", range);

        let result = self
            .client
            .spreadsheets()
            .values_update(req, sheet_id, range)
            .include_values_in_response(false)
            .value_input_option("RAW")
            .doit()
            .await;

        return match result {
            Err(e) => match e {
                Error::Failure(res) => Err(IError::new_internal(
                    String::from("write"),
                    String::from("failed to get cells"),
                    read_response_body(res).await?,
                )),
                // The Error enum provides details about what exactly> happened.
                // You can also just use its `Debug`, `Display` or `Error` traits
                Error::HttpError(_)
                | Error::Io(_)
                | Error::MissingAPIKey
                | Error::MissingToken(_)
                | Error::Cancelled
                | Error::UploadSizeLimitExceeded(_, _)
                | Error::BadRequest(_)
                | Error::FieldClash(_)
                | Error::JsonDecodeError(_, _) => Err(IError::new_internal(
                    String::from("write"),
                    String::from("internal spreadsheet error"),
                    e.to_string(),
                )),
            },
            Ok(_) => Ok(()),
        };
    }
}

impl APIWrapper {
    // returns new instance of APIWrapper.
    pub async fn new_with_init() -> APIWrapper {
        debug!("reading credentials");

        // Get an ApplicationSecret instance by some means. It contains the `client_id` and
        // `client_secret`, among other things.
        let secret = yup_oauth2::read_application_secret("credentials.json")
            .await
            .expect("client secret could not be read");

        debug!("creating auth");
        // Instantiate the authenticator. It will choose a suitable authentication flow for you,
        // unless you replace  `None` with the desired Flow.
        // Provide your own `AuthenticatorDelegate` to adjust the way it operates and get feedback about
        // what's going on. You probably want to bring in your own `TokenStorage` to persist tokens and
        // retrieve them from storage.
        let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
            secret,
            yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk("tokencache.json")
        .build()
        .await
        .unwrap();

        debug!("creating hub");
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http1()
            .build();
        let client = Sheets::new(hyper::Client::builder().build(https), auth);
        APIWrapper { client }
    }

    // get returns cell value from a sheet.
    pub async fn get(&self, sheet_id: &str, range: &str) -> Result<Vec<String>> {
        let result = self
            .client
            .spreadsheets()
            .values_get(sheet_id, range)
            .doit()
            .await;

        match result {
            Err(e) => match e {
                Error::Failure(res) => Err(IError::new_internal(
                    String::from("get"),
                    String::from("failed to get cells"),
                    read_response_body(res).await?,
                )),
                // The Error enum provides details about what exactly> happened.
                // You can also just use its `Debug`, `Display` or `Error` traits
                Error::HttpError(_)
                | Error::Io(_)
                | Error::MissingAPIKey
                | Error::MissingToken(_)
                | Error::Cancelled
                | Error::UploadSizeLimitExceeded(_, _)
                | Error::BadRequest(_)
                | Error::FieldClash(_)
                | Error::JsonDecodeError(_, _) => Err(IError::new_internal(
                    String::from("get"),
                    String::from("internal spreadsheet error"),
                    e.to_string(),
                )),
            },
            // we need to unwrap this 2d array properly here.
            Ok(vr) => Ok(vr
                .1
                .values
                .unwrap()
                .into_iter()
                .map(|v| v[0].clone())
                .collect()),
        }
    }
}

// read_response_body
async fn read_response_body(res: Response<Body>) -> Result<String> {
    let bytes = body::to_bytes(res.into_body()).await.map_err(|err| {
        IError::new_internal(
            String::from("read_response_body"),
            String::from("failed to convert body to bytes"),
            err.to_string(),
        )
    })?;
    String::from_utf8(bytes.to_vec()).map_err(|err| {
        IError::new_internal(
            String::from("read_response_body"),
            String::from("failed to convert body to String"),
            err.to_string(),
        )
    })
}

#[derive(Default)]
pub struct StdoutAPI {}

#[async_trait]
impl API for StdoutAPI {
    async fn write(&self, values: Vec<Vec<String>>, sheet_id: &str, range: &str) -> Result<()> {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        info!("{:?} {} {}", values, sheet_id, range);
        Ok(())
    }
}

pub struct TestAPI {
    sender: Sender<Vec<Vec<String>>>,
}

impl TestAPI {
    pub fn new() -> (Self, Receiver<Vec<Vec<String>>>) {
        let (sender, receiver) = channel::<Vec<Vec<String>>>(1);
        (Self { sender }, receiver)
    }
}

#[async_trait]
impl API for TestAPI {
    async fn write(&self, values: Vec<Vec<String>>, _sheet_id: &str, _range: &str) -> Result<()> {
        self.sender.send(values).await.map_err(|err| {
            IError::new_internal(
                String::from("TestAPI:write"),
                String::from("failed to send values to receiver"),
                err.to_string(),
            )
        })
    }
}

pub enum APIType {
    STDOUT, // indicates API that only prints content.
    SHEETS, // indicates API that writes data to google sheets.
}

pub async fn api_factory(api_type: APIType) -> Box<dyn API> {
    match api_type {
        APIType::STDOUT => Box::new(StdoutAPI::default()),
        APIType::SHEETS => Box::new(APIWrapper::new_with_init().await),
    }
}
