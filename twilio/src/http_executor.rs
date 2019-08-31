use futures::future::Future;
use reqwest::r#async::Client;
use std::fmt;
use std::io::Error;
use std::io::ErrorKind::Other;
use std::os::unix::process::ExitStatusExt;
use std::process::Output;

use crate::GenericResponseFuture;
use crate::SMSExecutor;

pub struct HTTPExecutor;

impl fmt::Display for HTTPExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HttpExecutor")
    }
}

impl SMSExecutor for HTTPExecutor {
    fn execute(
        &self,
        from: &str,
        to: &str,
        body: &str,
        account_id: &str,
        access_token: &str,
    ) -> GenericResponseFuture {
        let client = Client::new();
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            account_id
        );
        let res = client
            .post(&url)
            .form(&[("Body", body), ("From", from), ("To", to)])
            .basic_auth(account_id, Some(access_token))
            .send();
        Box::new(
            res.and_then(|mut r| r.text())
                .map(|t| Output {
                    stdout: t.into_bytes(),
                    stderr: vec![],
                    status: ExitStatusExt::from_raw(0),
                })
                .map_err(|e| Error::new(Other, e)),
        )
    }
}
