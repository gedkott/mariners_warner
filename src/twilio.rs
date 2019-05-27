use std::process::Command;

use serde::{Deserialize, Serialize};

pub fn send_text_message(
    from: &str,
    to: &str,
    account_id: &str,
    access_token: &str,
) -> TwilioResponse {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "
curl -X POST https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json \
    --data-urlencode \"Body=A Mariners Home Game is Starting\" \
    --data-urlencode \"From=+1{}\" \
    --data-urlencode \"To=+1{}\" \
    -u {}:{}
            ",
            account_id, from, to, account_id, access_token
        ))
        .output()
        .expect("failed to execute process");
    let response: TwilioResponse =
        serde_json::from_str(&String::from_utf8(output.stdout.to_vec()).unwrap()).unwrap();
    response
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let data = r#"
            {
                "account_sid": "ABCD1234",
                "api_version": "2010-04-01",
                "body": "Hello! 👍",
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

        let response: TwilioResponse = match serde_json::from_str(data) {
            Ok(r) => r,
            Err(e) => {
                println!("{}", e);
                panic!()
            }
        };
        assert_eq!(TwilioResponse::SendMessage {
            account_sid: "ABCD1234".to_string(),
            api_version: "2010-04-01".to_string(),
            body: "Hello! 👍".to_string(),
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
