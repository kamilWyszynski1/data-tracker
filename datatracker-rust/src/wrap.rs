use hyper::{body, Body, Response};
use sheets4::api::ValueRange;
use sheets4::{Error, Sheets};

pub struct APIWrapper {
    client: sheets4::Sheets,
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
        let client = Sheets::new(
            hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
            auth,
        );
        APIWrapper { client }
    }

    // writes data to a sheet.
    pub async fn write(
        &self,
        values: Vec<Vec<String>>,
        sheet_id: &str,
        range: &str,
    ) -> Result<(), String> {
        let mut req = ValueRange::default();
        req.values = Some(values);

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
                Error::Failure(res) => Err(read_response_body(res).await.unwrap()),
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
                | Error::JsonDecodeError(_, _) => Err(format!("{:?}", e)),
            },
            Ok(_) => Ok(()),
        };
    }
}

// read_response_body
async fn read_response_body(res: Response<Body>) -> Result<String, hyper::Error> {
    let bytes = body::to_bytes(res.into_body()).await?;
    Ok(String::from_utf8(bytes.to_vec()).expect("response was not valid utf-8"))
}
