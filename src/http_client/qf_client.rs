use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use serde_json::json;
use tokio::sync::{broadcast::Receiver as BReceiver, Mutex};

use crate::{block_in_place, AppError, StopSignal};

use super::{
    auth_state::AuthState,
    client::{ArcClientHandle, ClientHandle, HttpClient, Request, RequestBuilder, Response},
};

#[derive(Debug)]
pub struct QFClient {
    pub endpoint: String,
    auth: Arc<Mutex<AuthState>>,
    client_handle: Option<ArcClientHandle>,
    stop_signal: BReceiver<StopSignal>,
}

impl HttpClient for QFClient {
    async fn sender_fn(
        &mut self,
        rq: RequestBuilder,
    ) -> Result<(ArcClientHandle, RequestBuilder), AppError> {
        let auth_mutex = self.auth.lock().await;
        let auth = auth_mutex.deref();
        let rq = if !auth.qf_access_token.is_empty() {
            rq.header(
                format!("Authorization: JWT {}", auth.qf_access_token)
                    .parse()
                    .expect("infallible"),
            )
        } else {
            rq
        };
        drop(auth_mutex);
        let client_handle = if let Some(handle) = self.client_handle.clone() {
            handle
        } else {
            let (request_sender, request_receiver) = tokio::sync::mpsc::channel::<Request>(1);
            let (respones_sender, response_receiver) = tokio::sync::mpsc::channel::<Response>(1);
            let stop_signal = self.stop_signal.resubscribe();
            let handle = ClientHandle::new(stop_signal.resubscribe())
                .port(443)
                .addr("https://api.quantframe.app/")
                .map_err(|e| AppError::new(e.to_string(), "send_request".to_string()))?
                .timeout(Duration::from_secs(5))
                .send_channel(request_sender)
                .receive_channel(response_receiver)
                .start_client(request_receiver, respones_sender);
            let handle = Arc::new(Mutex::new(handle));
            self.client_handle = Some(handle.clone());
            handle
        };
        Ok((client_handle, rq))
    }

    async fn rate_limit(&self) {}
}

impl QFClient {
    pub fn new(auth: Arc<Mutex<AuthState>>, stop_signal: BReceiver<StopSignal>) -> Self {
        Self {
            endpoint: String::from("https://api.quantframe.app/"),
            auth,
            client_handle: None,
            stop_signal,
        }
    }
    pub async fn login(
        &mut self,
        id: Arc<str>,
        check_code: Arc<str>,
        ingame_name: Arc<str>,
    ) -> Result<(), AppError> {
        let req = RequestBuilder::new()
            .method(super::client::Method::POST)
            .uri(format!("{}auth/login", self.endpoint).as_str())
            .header("Device: thingamajig".parse().unwrap())
            .header("Content-Type: application/json".parse().unwrap())
            .header("Accept: */*".parse().unwrap())
            .body(json!({
                "username": id,
                "password": check_code,
                "current_version": "1.2.5",
                "ingame_name": ingame_name
            }))
            .build();
        let res = block_in_place!(async { self.send_request(req).await })
            .map_err(|e| e.prop("login".into()))?;
        let value = res.res.0.expect("body should be some")["token"].clone();
        let token = value.as_str().expect("token should be a string");
        let mut auth = self.auth.lock().await;
        let auth = auth.deref_mut();
        auth.qf_access_token = token.into();
        auth.update().map_err(|e| e.prop("login".into()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use tokio::sync::{broadcast, Mutex};

    use crate::{
        block_in_place,
        http_client::{
            auth_state::AuthState,
            client::{HttpClient, Method, RequestBuilder},
        },
    };

    use super::QFClient;

    #[test]
    fn test_qfclient() {
        let auth = AuthState::setup().unwrap();
        let auth = Arc::new(Mutex::new(auth));
        let (stop_sender, _) = broadcast::channel(1);
        let mut client = QFClient::new(auth, stop_sender.subscribe());
        let _ = block_in_place!(async move {
            let req = RequestBuilder::new()
                .method(Method::GET)
                .uri(client.endpoint.as_str())
                .build();
            client.send_request(req).await
        });
    }
}
