//! Use Caseからシステム時刻を分離するClock port。

pub trait Clock {
    fn now_utc_iso8601(&self) -> String;
}
