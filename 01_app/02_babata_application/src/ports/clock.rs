use babata_domain::UtcTimestamp;

pub trait ClockPort {
    fn now(&self) -> UtcTimestamp;
}
