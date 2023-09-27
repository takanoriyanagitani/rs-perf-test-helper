use core::time::Duration;

use tonic::Status;

use crate::rpc::perf::helper;
use helper::proto::common::v1::Retry as Gretry;

pub const INTERVAL_DEFAULT: Duration = Duration::from_micros(100);
pub const INTERVAL_MIN: Duration = Duration::from_nanos(1);
pub const TIMEOUT_DEFAULT: Duration = Duration::from_millis(100);

pub struct Retry {
    retry_max: u64,
    interval: Duration,
    timeout: Duration,
}

impl Retry {
    pub fn as_retry_max(&self) -> u64 {
        self.retry_max
    }
    pub fn as_interval(&self) -> Duration {
        self.interval.max(INTERVAL_MIN)
    }
    pub fn as_timeout(&self) -> Duration {
        self.timeout
    }
}

pub trait RetryLike {
    fn as_retry_max(&self) -> u64;
    fn as_interval(&self) -> Option<Duration>;
    fn as_timeout(&self) -> Option<Duration>;
}

impl RetryLike for Gretry {
    fn as_retry_max(&self) -> u64 {
        self.retry_max
    }
    fn as_interval(&self) -> Option<Duration> {
        self.interval.clone().and_then(|d| d.try_into().ok())
    }
    fn as_timeout(&self) -> Option<Duration> {
        self.timeout.clone().and_then(|d| d.try_into().ok())
    }
}

impl<R> From<&R> for Retry
where
    R: RetryLike,
{
    fn from(r: &R) -> Self {
        let retry_max: u64 = r.as_retry_max();
        let interval: Duration = r.as_interval().unwrap_or(INTERVAL_DEFAULT);
        let timeout: Duration = r.as_timeout().unwrap_or(TIMEOUT_DEFAULT);
        Self {
            retry_max,
            interval,
            timeout,
        }
    }
}

impl<R> TryFrom<Option<&R>> for Retry
where
    R: RetryLike,
{
    type Error = Status;
    fn try_from(o: Option<&R>) -> Result<Self, Self::Error> {
        let r: &R = o.ok_or_else(|| Status::invalid_argument("retry missing"))?;
        Ok(r.into())
    }
}
