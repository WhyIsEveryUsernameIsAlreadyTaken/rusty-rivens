use std::{
    sync::Arc, thread::JoinHandle, time::Duration
};

use async_lock::Mutex;

use crate::rate_limiter::RateLimiter;

use super::client::HttpClient;

#[derive(Clone, Debug)]
pub struct QFClient {
    pub endpoint: String,
    pub limiter: Arc<Mutex<RateLimiter>>,
}

impl<'a> HttpClient<'a> for QFClient {}

impl QFClient {
    pub fn new() -> Self {
        Self {
            endpoint: String::from("https://api.quantframe.app/items/riven/raw"),
            limiter: Arc::new(Mutex::new(RateLimiter::new(1.0, Duration::new(1, 0)))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ops::{Deref, DerefMut}, time::Duration};

    use crate::{http_client::client::{HttpClient, Method}, rate_limiter::{self, RateLimiter}, STOPPED};

    use super::QFClient;

    #[test]
    fn test_qfclient() {
        let client = QFClient::new();
        let req = smolscale::block_on(async move {
            let mut limiter = client.limiter.lock().await;
            client.send_request(
                Method::GET,
                client.endpoint.as_str(),
                limiter.deref_mut(),
                None,
                None
            ).await
        });
        let _ = STOPPED.set(true);
    }
}
