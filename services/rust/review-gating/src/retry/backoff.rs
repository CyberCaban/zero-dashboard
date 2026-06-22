use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub factor: f64,
    pub max_retries: u32,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            factor: 2.0,
            max_retries: 3,
        }
    }
}

impl ExponentialBackoff {
    pub fn iter(&self) -> BackoffIter {
        BackoffIter {
            current_delay: self.initial_delay,
            max_delay: self.max_delay,
            factor: self.factor,
            retries: 0,
            max_retries: self.max_retries,
        }
    }
}

pub struct BackoffIter {
    current_delay: Duration,
    max_delay: Duration,
    factor: f64,
    retries: u32,
    max_retries: u32,
}

impl Iterator for BackoffIter {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.retries >= self.max_retries {
            return None;
        }

        self.retries += 1;

        let delay = self.current_delay;

        let next_delay =
            (self.current_delay.as_secs_f64() * self.factor).min(self.max_delay.as_secs_f64());
        self.current_delay = Duration::from_secs_f64(next_delay);

        let jitter: f64 = rand::random::<f64>() * 0.1;
        let delay_with_jitter = delay.as_secs_f64() * (1.0 + jitter);

        Some(Duration::from_secs_f64(delay_with_jitter))
    }
}
