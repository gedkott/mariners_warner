use std::fmt::Display;
use std::process::Command;

use futures::future::result;
use futures::Future;
use serde::{Deserialize, Serialize};
use serde_json::from_str;

mod http_executor;

pub use self::http_executor::HTTPExecutor;

#[derive(Clone)]
pub struct TwilioConfig {
    pub from: String,
    pub to: String,
    pub twilio_account_id: String,
    pub twilio_access_token: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum TwilioResponse {
    SendMessage {
        account_sid: String,
        api_version: String,
        body: String,
        date_created: String,
        date_sent: Option<String>,
        date_updated: String,
        direction: String,
        error_code: Option<String>,
        error_message: Option<String>,
        from: String,
        messaging_service_sid: Option<String>,
        num_media: String,
        num_segments: String,
        price: Option<f32>,
        price_unit: String,
        sid: String,
        status: String,
        subresource_uris: SubresourceURIs,
        to: String,
        uri: String,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SubresourceURIs {
    media: String,
}

pub type GenericResponseFuture =
    Box<dyn Future<Item = std::process::Output, Error = std::io::Error> + Send>;
pub type TwilioResponseFuture = Box<dyn Future<Item = TwilioResponse, Error = SMSError> + Send>;

#[derive(Debug, PartialEq)]
pub enum SMSError {
    TwilioResponseError { error: String },
    SerdeError { error: String, raw_response: String },
    ExecutionError { error: String },
}

pub trait SMSExecutor {
    fn execute(
        &self,
        from: &str,
        to: &str,
        body: &str,
        account_id: &str,
        access_token: &str,
    ) -> GenericResponseFuture;
}

pub fn send_text_message(
    from: &str,
    to: &str,
    account_id: &str,
    access_token: &str,
    text_content: &str,
    sms_executor: &SMSExecutor,
) -> TwilioResponseFuture {
    let twilio_response = sms_executor
        .execute(from, to, text_content, account_id, access_token)
        .map_err(|error| SMSError::ExecutionError {
            error: error.to_string(),
        })
        .and_then(convert_to_string)
        .and_then(|response| deserialize_twilio_response(&response));
    Box::new(twilio_response)
}

fn convert_to_string(output: std::process::Output) -> Result<String, SMSError> {
    String::from_utf8(output.stdout).map_err(|error| SMSError::TwilioResponseError {
        error: error.to_string(),
    })
}

fn deserialize_twilio_response(response: &str) -> Result<TwilioResponse, SMSError> {
    from_str(response).map_err(|error| SMSError::SerdeError {
        error: error.to_string(),
        raw_response: response.to_string(),
    })
}

pub struct CommandExecutor;

impl Display for CommandExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CommandExecutor")
    }
}

impl SMSExecutor for CommandExecutor {
    fn execute(
        &self,
        from: &str,
        to: &str,
        body: &str,
        account_id: &str,
        access_token: &str,
    ) -> GenericResponseFuture {
        Box::new(result(
            Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "
        curl -X POST https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json \
            --data-urlencode \"Body={}\" \
            --data-urlencode \"From=+1{}\" \
            --data-urlencode \"To=+1{}\" \
            -u {}:{}
                    ",
                    account_id, body, from, to, account_id, access_token
                ))
                .output(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::{err, ok};

    struct SuccesfulMockExecutor;
    struct FailingMockExecutor;

    impl SMSExecutor for SuccesfulMockExecutor {
        fn execute(
            &self,
            _: &str,
            _: &str,
            body: &str,
            _: &str,
            _: &str,
        ) -> Box<Future<Item = std::process::Output, Error = std::io::Error> + Send> {
            let data = r#"
            {
                "account_sid": "ABCD1234",
                "api_version": "2010-04-01",
                "body": "{body}",
                "date_created": "Thu, 30 Jul 2015 20:12:31 +0000",
                "date_sent": "Thu, 30 Jul 2015 20:12:33 +0000",
                "date_updated": "Thu, 30 Jul 2015 20:12:33 +0000",
                "direction": "outbound-api",
                "error_code": null,
                "error_message": null,
                "from": "+14155552345",
                "messaging_service_sid": "MGXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                "num_media": "0",
                "num_segments": "1",
                "price": -0.00750,
                "price_unit": "USD",
                "sid": "MMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                "status": "sent",
                "subresource_uris": {
                    "media": "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/Media.json"
                },
                "to": "+14155552345",
                "uri": "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX.json"
            }"#;

            let data = data.replace("{body}", body);

            Box::new(ok(std::process::Output {
                stdout: data.as_bytes().to_vec(),
                stderr: vec![],
                status: std::os::unix::process::ExitStatusExt::from_raw(0),
            }))
        }
    }

    impl SMSExecutor for FailingMockExecutor {
        fn execute(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
        ) -> Box<Future<Item = std::process::Output, Error = std::io::Error> + Send> {
            let custom_error = std::io::Error::new(std::io::ErrorKind::Other, "oh no!");
            Box::new(err(custom_error))
        }
    }

    #[test]
    fn different_executor_works_with_success_flow() {
        let response = send_text_message(
            "1234567890",
            "0987654321",
            "ABCD1234",
            "A1B2C3D4",
            "SUPPP",
            &SuccesfulMockExecutor,
        )
        .wait()
        .unwrap();
        assert_eq!(TwilioResponse::SendMessage {
            account_sid: "ABCD1234".to_string(),
            api_version: "2010-04-01".to_string(),
            body: "SUPPP".to_string(),
            date_created: "Thu, 30 Jul 2015 20:12:31 +0000".to_string(),
            date_sent: Some("Thu, 30 Jul 2015 20:12:33 +0000".to_string()),
            date_updated: "Thu, 30 Jul 2015 20:12:33 +0000".to_string(),
            direction: "outbound-api".to_string(),
            error_code: None,
            error_message: None,
            from: "+14155552345".to_string(),
            messaging_service_sid: Some("MGXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string()),
            num_media: "0".to_string(),
            num_segments: "1".to_string(),
            price: Some(-0.00750),
            price_unit: "USD".to_string(),
            sid: "MMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            status: "sent".to_string(),
            subresource_uris: SubresourceURIs {
                media: "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/Media.json".to_string()
            },
            to: "+14155552345".to_string(),
            uri: "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX.json".to_string()
        }, response);
    }

    #[test]
    fn different_executor_works_with_fail_flow() {
        let response = send_text_message(
            "1234567890",
            "0987654321",
            "ABCD1234",
            "A1B2C3D4",
            "SUPPP",
            &FailingMockExecutor,
        )
        .wait()
        .err();
        assert_eq!(
            Some(SMSError::ExecutionError {
                error: "oh no!".to_string(),
            }),
            response
        );
    }

    #[test]
    fn it_works() {
        let data = r#"
            {
                "account_sid": "ABCD1234",
                "api_version": "2010-04-01",
                "body": "HI!",
                "date_created": "Thu, 30 Jul 2015 20:12:31 +0000",
                "date_sent": "Thu, 30 Jul 2015 20:12:33 +0000",
                "date_updated": "Thu, 30 Jul 2015 20:12:33 +0000",
                "direction": "outbound-api",
                "error_code": null,
                "error_message": null,
                "from": "+14155552345",
                "messaging_service_sid": "MGXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                "num_media": "0",
                "num_segments": "1",
                "price": -0.00750,
                "price_unit": "USD",
                "sid": "MMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
                "status": "sent",
                "subresource_uris": {
                    "media": "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/Media.json"
                },
                "to": "+14155552345",
                "uri": "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX.json"
            }"#;

        let response: TwilioResponse = match from_str(data) {
            Ok(r) => r,
            Err(e) => {
                println!("{}", e);
                panic!()
            }
        };
        assert_eq!(TwilioResponse::SendMessage {
            account_sid: "ABCD1234".to_string(),
            api_version: "2010-04-01".to_string(),
            body: "HI!".to_string(),
            date_created: "Thu, 30 Jul 2015 20:12:31 +0000".to_string(),
            date_sent: Some("Thu, 30 Jul 2015 20:12:33 +0000".to_string()),
            date_updated: "Thu, 30 Jul 2015 20:12:33 +0000".to_string(),
            direction: "outbound-api".to_string(),
            error_code: None,
            error_message: None,
            from: "+14155552345".to_string(),
            messaging_service_sid: Some("MGXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string()),
            num_media: "0".to_string(),
            num_segments: "1".to_string(),
            price: Some(-0.00750),
            price_unit: "USD".to_string(),
            sid: "MMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string(),
            status: "sent".to_string(),
            subresource_uris: SubresourceURIs {
                media: "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/Media.json".to_string()
            },
            to: "+14155552345".to_string(),
            uri: "/2010-04-01/Accounts/ABCD1234/Messages/SMXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX.json".to_string()
        }, response);
    }
}
