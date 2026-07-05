#![allow(dead_code)]

pub trait Clock {
    fn now_utc_iso8601(&self) -> String;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc_iso8601(&self) -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }
}
