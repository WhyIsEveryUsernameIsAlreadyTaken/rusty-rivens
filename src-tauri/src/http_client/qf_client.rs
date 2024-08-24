use std::{
    sync::Arc,
    time::Duration,
};

use futures::lock::Mutex;

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
