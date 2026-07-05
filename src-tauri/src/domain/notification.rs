#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationDisplayMode {
    TitleOnly,
    Generic,
}

impl NotificationDisplayMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TitleOnly => "title_only",
            Self::Generic => "generic",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "title_only" => Ok(Self::TitleOnly),
            "generic" => Ok(Self::Generic),
            _ => Err(format!("不正な通知表示モードです: {value}")),
        }
    }
}

pub fn build_notification_body(mode: &NotificationDisplayMode, title: &str) -> String {
    match mode {
        NotificationDisplayMode::TitleOnly => title.trim().to_string(),
        NotificationDisplayMode::Generic => "TaskTimerの予定時刻です".to_string(),
    }
}
