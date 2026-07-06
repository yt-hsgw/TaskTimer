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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationKind {
    PlannedStart,
    Due,
}

impl NotificationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlannedStart => "planned_start",
            Self::Due => "due",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "planned_start" => Ok(Self::PlannedStart),
            "due" => Ok(Self::Due),
            _ => Err(format!("不正な通知種別です: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationRegistrationStatus {
    Pending,
    Registered,
    Failed,
    Disabled,
}

impl NotificationRegistrationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Registered => "registered",
            Self::Failed => "failed",
            Self::Disabled => "disabled",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "registered" => Ok(Self::Registered),
            "failed" => Ok(Self::Failed),
            "disabled" => Ok(Self::Disabled),
            _ => Err(format!("不正な通知登録状態です: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationContent {
    pub title: String,
    pub body: String,
}

pub fn build_notification_content(
    mode: &NotificationDisplayMode,
    target_title: &str,
) -> NotificationContent {
    let sanitized_title = target_title.trim();
    match mode {
        NotificationDisplayMode::TitleOnly => NotificationContent {
            title: sanitized_title.to_string(),
            body: String::new(),
        },
        NotificationDisplayMode::Generic => NotificationContent {
            title: "TaskTimer".to_string(),
            body: "予定時刻です".to_string(),
        },
    }
}
