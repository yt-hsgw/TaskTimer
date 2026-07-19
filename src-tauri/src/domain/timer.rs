#![allow(dead_code)]

pub const DEFAULT_TASK_TIMER_SETTINGS_ID: &str = "default";
pub const DEFAULT_TASK_TIMER_TARGET_SECONDS: i64 = 30 * 60;
pub const MIN_TASK_TIMER_TARGET_SECONDS: i64 = 60;
pub const MAX_TASK_TIMER_TARGET_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkTargetType {
    Task,
    Subtask,
}

impl WorkTargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Subtask => "subtask",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "task" => Ok(Self::Task),
            "subtask" => Ok(Self::Subtask),
            _ => Err(format!("不正な対象種別です: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerCompletionReason {
    Manual,
    CountdownExpired,
}

impl TimerCompletionReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::CountdownExpired => "countdown_expired",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "manual" => Ok(Self::Manual),
            "countdown_expired" => Ok(Self::CountdownExpired),
            _ => Err(format!("不正なタイマー完了理由です: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkTargetRef {
    pub target_type: WorkTargetType,
    pub id: String,
}
