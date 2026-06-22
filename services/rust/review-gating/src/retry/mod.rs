use tracing::{error, info, warn};

pub mod backoff;

pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    backoff: backoff::ExponentialBackoff,
    operation_name: &str,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut last_error: Option<E> = None;
    for delay in backoff.iter() {
        attempts += 1;
        match operation().await {
            Ok(result) => {
                if attempts > 1 {
                    info!(
                        operation = operation_name,
                        attempts = attempts,
                        "Operation succeeded after {} attempts",
                        attempts
                    );
                }
                return Ok(result);
            }
            Err(err) => {
                warn!(
                    operation = operation_name,
                    attempts = attempts,
                    error = %err,
                    retry_after_ms = delay.as_millis(),
                    "Operation failed, retrying",
                );
                last_error = Some(err);
                tokio::time::sleep(delay).await;
            }
        }
    }

    error!(
        operation = operation_name,
        attempts = attempts,
        "Operation failed after maximum retry attempts"
    );
    Err(last_error.expect("Expected at least one error"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn test_retry_with_backoff() {
        let attempts = Arc::new(Mutex::new(0));
        let operation = || {
            let attempts = attempts.clone();
            async move {
                let mut lock = attempts.lock().unwrap();
                *lock += 1;
                if *lock < 3 {
                    Err("Failed")
                } else {
                    Ok("Success")
                }
            }
        };

        let backoff = backoff::ExponentialBackoff {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(200),
            factor: 2.0,
            max_retries: 5,
        };

        let result = retry_with_backoff(operation, backoff, "test_operation").await;
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(*attempts.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_exhausted() {
        let attempts = Arc::new(Mutex::new(0));
        let operation = || {
            let attempts = attempts.clone();
            async move {
                let mut lock = attempts.lock().unwrap();
                *lock += 1;
                Err::<i32, &str>("Failed")
            }
        };

        let backoff = backoff::ExponentialBackoff {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(200),
            factor: 2.0,
            max_retries: 3,
        };

        let result = retry_with_backoff(operation, backoff, "test_operation").await;
        assert!(result.is_err());
        assert_eq!(*attempts.lock().unwrap(), 3);
    }
}
