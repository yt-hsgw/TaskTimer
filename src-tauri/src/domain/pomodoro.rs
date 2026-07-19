#![allow(dead_code)]

pub const DEFAULT_POMODORO_SETTINGS_ID: &str = "default";
pub const DEFAULT_POMODORO_WORK_SECONDS: i64 = 25 * 60;
pub const DEFAULT_POMODORO_SHORT_BREAK_SECONDS: i64 = 5 * 60;
pub const DEFAULT_POMODORO_LONG_BREAK_SECONDS: i64 = 15 * 60;
pub const DEFAULT_POMODORO_CYCLES_UNTIL_LONG_BREAK: i64 = 4;
pub const POMODORO_DURATION_MIN_SECONDS: i64 = 60;
pub const POMODORO_DURATION_MAX_SECONDS: i64 = 24 * 60 * 60;
pub const POMODORO_CYCLES_MIN: i64 = 1;
pub const POMODORO_CYCLES_MAX: i64 = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PomodoroScope {
    TaskLinked,
    Standalone,
}

impl PomodoroScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TaskLinked => "task_linked",
            Self::Standalone => "standalone",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "task_linked" => Ok(Self::TaskLinked),
            "standalone" => Ok(Self::Standalone),
            _ => Err(format!("不正なポモドーロscopeです: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PomodoroPhase {
    Work,
    ShortBreak,
    LongBreak,
}

impl PomodoroPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::ShortBreak => "short_break",
            Self::LongBreak => "long_break",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "work" => Ok(Self::Work),
            "short_break" => Ok(Self::ShortBreak),
            "long_break" => Ok(Self::LongBreak),
            _ => Err(format!("不正なポモドーロフェーズです: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PomodoroStatus {
    Running,
    Paused,
    Completed,
    Cancelled,
}

impl PomodoroStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("不正なポモドーロ状態です: {value}")),
        }
    }
}

pub fn validate_pomodoro_duration_seconds(value: i64, field_label: &str) -> Result<i64, String> {
    if !(POMODORO_DURATION_MIN_SECONDS..=POMODORO_DURATION_MAX_SECONDS).contains(&value) {
        return Err(format!(
            "{field_label}は{}秒以上{}秒以下で入力してください",
            POMODORO_DURATION_MIN_SECONDS, POMODORO_DURATION_MAX_SECONDS
        ));
    }
    Ok(value)
}

pub fn validate_pomodoro_cycles_until_long_break(value: i64) -> Result<i64, String> {
    if !(POMODORO_CYCLES_MIN..=POMODORO_CYCLES_MAX).contains(&value) {
        return Err(format!(
            "長い休憩までの作業回数は{POMODORO_CYCLES_MIN}以上{POMODORO_CYCLES_MAX}以下で入力してください"
        ));
    }
    Ok(value)
}

pub fn next_break_phase(completed_work_count: i64, cycles_until_long_break: i64) -> PomodoroPhase {
    if completed_work_count > 0 && completed_work_count % cycles_until_long_break == 0 {
        PomodoroPhase::LongBreak
    } else {
        PomodoroPhase::ShortBreak
    }
}
