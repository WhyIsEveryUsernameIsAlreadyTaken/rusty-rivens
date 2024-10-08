use once_cell::sync::OnceCell;

use super::client::HttpClient;

#[derive(Clone, Debug)]
pub struct QFClient {
    pub endpoint: String,
}

impl<'a> HttpClient<'a> for QFClient {}

impl QFClient {
    pub fn new() -> Self {
        Self {
            endpoint: String::from("https://api.quantframe.app/items/riven/raw"),
        }
    }
}

pub static TEST_QF_STOPPED: OnceCell<bool> = once_cell::sync::OnceCell::new();

#[cfg(test)]
mod tests {

    use crate::http_client::client::{HttpClient, Method};

    use super::{QFClient, TEST_QF_STOPPED};

    #[test]
    fn test_qfclient() {
        let client = QFClient::new();
        let _ = smolscale::block_on(async move {
            client.send_request(
                Method::GET,
                client.endpoint.as_str(),
                &mut None,
                None,
                None
            ).await
        });
        TEST_QF_STOPPED.set(true).unwrap();
    }
}
