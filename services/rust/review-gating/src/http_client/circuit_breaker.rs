use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use tokio::{
    sync::{Mutex, RwLock},
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal State
    Closed,
    /// Blocking State
    Open,
    /// 'Time to check' State
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    failure_threshold: u32,
    timeout: Duration,
    last_error_time: Arc<Mutex<Option<Instant>>>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        CircuitBreaker::new(5, Duration::from_secs(30))
    }
}
impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            failure_threshold,
            timeout,
            last_error_time: Arc::new(Mutex::new(None)),
        }
    }
    pub async fn get_state(&self) -> CircuitState {
        {
            let mut state = self.state.write().await;
            if *state == CircuitState::Open {
                let last_error = *self.last_error_time.lock().await;
                if let Some(time) = last_error {
                    if time.elapsed() >= self.timeout {
                        *state = CircuitState::HalfOpen;
                        tracing::info!("Circuit breaker HALF OPENED");
                    }
                }
                tracing::warn!("Circuit breaker is OPEN, blocking request");
            }
        }
        *self.state.read().await
    }
    pub async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_error_time.lock().await = Some(Instant::now());

        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => {
                if failures >= self.failure_threshold {
                    *state = CircuitState::Open;
                    tracing::error!(
                        failure_count = failures,
                        threshold = self.failure_threshold,
                        "Circuit breaker OPENED due to repeated failures"
                    );
                }
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open;
                tracing::error!("Circuit breaker reopened - service still failing");
            }
            CircuitState::Open => {}
        }
    }
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        self.failure_count.store(0, Ordering::SeqCst);

        if *state == CircuitState::HalfOpen {
            *state = CircuitState::Closed;
            tracing::info!("Circuit breaker CLOSED - service recovered")
        }
    }
    pub async fn is_request_allowed(&self) -> bool {
        let mut state = self.state.write().await;
        match *state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => {
                let last_error = *self.last_error_time.lock().await;
                if let Some(time) = last_error {
                    if time.elapsed() >= self.timeout {
                        *state = CircuitState::HalfOpen;
                        tracing::info!("Circuit breaker HALF OPENED");
                        return true;
                    }
                }
                tracing::warn!("Circuit breaker is OPEN, blocking request");
                false
            }
        }
    }
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::SeqCst);
        *self.last_error_time.lock().await = None;
        tracing::info!("Circuit breaker manually reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::sleep;

    fn create_breaker(failure_threshold: u32, timeout: Duration) -> Arc<CircuitBreaker> {
        Arc::new(CircuitBreaker::new(failure_threshold, timeout))
    }

    #[tokio::test]
    async fn test_initial_state_is_closed() {
        let breaker = create_breaker(3, Duration::from_secs(5));
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_requests_allowed_when_closed() {
        let breaker = create_breaker(3, Duration::from_secs(5));
        assert!(breaker.is_request_allowed().await);
    }

    #[tokio::test]
    async fn test_opens_after_failure_threshold_reached() {
        let breaker = create_breaker(3, Duration::from_secs(10));

        for _ in 0..3 {
            breaker.record_failure().await;
        }

        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.is_request_allowed().await);
    }

    #[tokio::test]
    async fn test_stays_closed_below_threshold() {
        let breaker = create_breaker(3, Duration::from_secs(10));

        for _ in 0..2 {
            breaker.record_failure().await;
        }

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.is_request_allowed().await);
    }

    #[tokio::test]
    async fn test_success_resets_failure_counter() {
        let breaker = create_breaker(3, Duration::from_secs(10));

        breaker.record_failure().await;
        breaker.record_failure().await;

        breaker.record_success().await;

        breaker.record_failure().await;
        breaker.record_failure().await;

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.is_request_allowed().await);
    }

    #[tokio::test]
    async fn test_half_open_after_timeout() {
        let breaker = create_breaker(2, Duration::from_millis(500));

        breaker.record_failure().await;
        breaker.record_failure().await;

        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.is_request_allowed().await);

        sleep(Duration::from_millis(600)).await;

        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);
        assert!(breaker.is_request_allowed().await);
    }

    #[tokio::test]
    async fn test_closes_on_success_in_half_open() {
        let breaker = create_breaker(2, Duration::from_millis(500));

        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        sleep(Duration::from_millis(600)).await;
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        assert!(breaker.is_request_allowed().await);
        breaker.record_success().await;

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.is_request_allowed().await);
    }

    #[tokio::test]
    async fn test_opens_again_on_failure_in_half_open() {
        let breaker = create_breaker(2, Duration::from_millis(500));

        // Close -> Open
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // Open -> HalfOpen
        sleep(Duration::from_millis(600)).await;
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);
        assert!(breaker.is_request_allowed().await);

        // HalfOpen -> Open
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.is_request_allowed().await);
    }

    #[tokio::test]
    async fn test_reset_works_from_any_state() {
        let breaker = create_breaker(2, Duration::from_secs(10));

        // Close -> Open
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // Reset
        breaker.reset().await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.is_request_allowed().await);

        // Close -> Open
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_failure_count_resets_on_state_change() {
        let breaker = create_breaker(2, Duration::from_millis(500));

        // Close -> Open
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // Open -> HalfOpen
        sleep(Duration::from_millis(600)).await;
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // HalfOpen -> Open
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // Open -> HalfOpen
        sleep(Duration::from_millis(600)).await;
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // HalfOpen -> Closed
        breaker.record_success().await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);

        // No transition
        breaker.record_failure().await;
        // Still Closed
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        breaker.record_failure().await;
        // Now Open
        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }
}
