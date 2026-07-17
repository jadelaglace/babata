#[derive(Debug, Clone)]
pub struct OperationLog {
    pub operation_id: String,
}

pub fn init_tracing() {}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl ClockPort for SystemClock {
    fn now(&self) -> UtcTimestamp {
        let value = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("RFC 3339 formatting is supported");
        UtcTimestamp::parse(value).expect("system UTC time is a valid timestamp")
    }
}

impl OperationLog {
    pub fn new(operation_id: String) -> Self {
        Self { operation_id }
    }
    pub fn event(&self, _event: &str) { /* Do not log raw content or configuration secrets. */
    }
}
use babata_application::ports::ClockPort;
use babata_domain::UtcTimestamp;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
