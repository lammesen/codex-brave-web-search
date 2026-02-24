use std::time::{Duration, Instant};

#[derive(Debug)]
struct BucketState {
    available_tokens: f64,
    last_refill: Instant,
}

#[derive(Debug)]
pub struct RequestThrottle {
    tokens_per_second: f64,
    burst_capacity: f64,
    state: tokio::sync::Mutex<BucketState>,
}

impl RequestThrottle {
    #[must_use]
    pub fn new(tokens_per_second: u32, burst_capacity: u32) -> Self {
        let rate = f64::from(tokens_per_second.max(1));
        let burst = f64::from(burst_capacity.max(1));
        Self {
            tokens_per_second: rate,
            burst_capacity: burst,
            state: tokio::sync::Mutex::new(BucketState {
                available_tokens: burst,
                last_refill: Instant::now(),
            }),
        }
    }

    pub async fn acquire(&self) {
        let _ = self.acquire_cancellable(&|| false).await;
    }

    pub async fn acquire_cancellable<F>(&self, is_cancelled: &F) -> Result<(), ()>
    where
        F: Fn() -> bool,
    {
        loop {
            if is_cancelled() {
                return Err(());
            }

            let mut state = self.state.lock().await;
            if is_cancelled() {
                return Err(());
            }

            let now = Instant::now();
            let elapsed = now.duration_since(state.last_refill).as_secs_f64();
            if elapsed > 0.0 {
                state.available_tokens = (state.available_tokens
                    + elapsed * self.tokens_per_second)
                    .min(self.burst_capacity);
                state.last_refill = now;
            }

            if state.available_tokens >= 1.0 {
                state.available_tokens -= 1.0;
                return Ok(());
            }

            let deficit = 1.0 - state.available_tokens;
            let wait_seconds = deficit / self.tokens_per_second;
            drop(state);

            let total_wait = Duration::from_secs_f64(wait_seconds.max(0.001));
            let start = Instant::now();
            let step = Duration::from_millis(20);
            while start.elapsed() < total_wait {
                if is_cancelled() {
                    return Err(());
                }
                let remaining = total_wait.saturating_sub(start.elapsed());
                tokio::time::sleep(remaining.min(step)).await;
            }
        }
    }
}
