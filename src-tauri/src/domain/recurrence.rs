#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
}

impl RecurrenceFrequency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            _ => Err(format!("不正な繰り返し頻度です: {value}")),
        }
    }
}
