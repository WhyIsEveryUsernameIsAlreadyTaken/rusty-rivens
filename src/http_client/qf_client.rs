use std::sync::Arc;

use once_cell::sync::OnceCell;
use tokio::sync::{mpsc::{channel, Receiver}, Mutex};

use super::client::{ClientHandle, HttpClient, Request, Response};

pub struct QFClient {
    pub endpoint: String,
    client_handle: Option<Arc<Mutex<ClientHandle>>>,
    response_receiver: Option<Arc<Mutex<Receiver<Response>>>>
}

impl<'a> HttpClient<'a> for QFClient {
    async fn send_fn(&mut self, req: super::client::RequestBuilder)
    -> Result<(Arc<Mutex<ClientHandle>>, Arc<Mutex<Receiver<Response>>>, super::client::RequestBuilder), crate::AppError> {
        let (client_handle, response_receiver) = if let Some(client_handle) = self.client_handle.clone() {
            (client_handle, self.response_receiver.clone().expect("reciever should be availabe after creating the client handle"))
        } else {
            let (request_sender, request_receiver) = channel::<Request>(1);
            let (response_sender, response_receiver) = channel::<Response>(1);
            let response_receiver_arc = Arc::new(Mutex::new(response_receiver));
            self.response_receiver = Some(response_receiver_arc.clone());
            let client_handle = ClientHandle::create(
                self.endpoint.as_str(),
                request_sender,
                request_receiver,
                response_sender
            ).map_err(|e| e.prop("QFClient::send_fn".into()))?;
            let client_handle = Arc::new(Mutex::new(client_handle));
            (client_handle,
            response_receiver_arc
        )
        };
        Ok((
            client_handle,
            response_receiver,
            req
        ))
    }
    async fn rate_limit(&self) {
    }
}

impl QFClient {
    pub fn new() -> Self {
        Self {
            endpoint: String::from("https://api.quantframe.app/items/riven/raw"),
            client_handle: None,
            response_receiver: None,
        }
    }
}

pub static TEST_QF_STOPPED: OnceCell<bool> = once_cell::sync::OnceCell::new();

#[cfg(test)]
mod tests {

    use crate::http_client::client::{HttpClient, Method, Request, RequestBuilder};

    use super::{QFClient, TEST_QF_STOPPED};

    #[tokio::test]
    async fn test_qfclient() {
        let mut client = QFClient::new();
        let req = Request::default();
        let _ = {
            let req = RequestBuilder::new()
                .method(Method::GET)
                .uri(client.endpoint.as_str())
                .build();
            client.send_request(req).await
        };
        TEST_QF_STOPPED.set(true).unwrap();
    }
}
