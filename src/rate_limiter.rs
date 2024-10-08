use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct RateLimiter {
    pub tokens: f64,
    last_timestamp: Instant,
    rate: f64,
    per: Duration,
    offset_secs: f64,
}

impl RateLimiter {
    pub fn new(rate: f64, per: Duration) -> Self {
        RateLimiter {
            tokens: rate,
            last_timestamp: Instant::now(),
            rate,
            per,
            offset_secs: 0.0,
        }
    }

    fn can_make_request(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_timestamp);
        let elapsed_secs = elapsed.as_secs_f64();

        // Refill tokens based on how much time has passed since the last request
        self.tokens += elapsed_secs / (self.per.as_secs_f64() * self.rate) + self.offset_secs;
        self.last_timestamp = now;

        self.offset_secs = 0.0;

        // Ensure the number of tokens doesn't exceed the rate
        self.tokens = self.tokens.min(self.rate);

        if self.tokens < 1.0 {
            false
        } else {
            // Remove a token and allow the request
            self.tokens -= 1.0;
            true
        }
    }

    pub fn add_delay(&mut self, delay_secs: f64) {
        self.offset_secs += delay_secs;
    }

    pub async fn wait_for_token(&mut self) {
        while !self.can_make_request() {
            let time_to_wait_secs =
                self.per.as_secs_f64() / self.rate - self.last_timestamp.elapsed().as_secs_f64();
            let time_to_wait = Duration::from_secs_f64(time_to_wait_secs);
            sleep(time_to_wait);
        }
    }
}
