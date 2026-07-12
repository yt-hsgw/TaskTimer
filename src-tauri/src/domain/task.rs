#![allow(dead_code)]

use time::{macros::format_description, Date, Time};

const DATE_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");
const TIME_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[hour]:[minute]");
const MEMO_MAX_CHARS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkStatus {
    Todo,
    InProgress,
    Done,
    Archived,
}

impl WorkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Archived => "archived",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "todo" => Ok(Self::Todo),
            "in_progress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("不正な状態値です: {value}")),
        }
    }
}

pub fn validate_title(title: &str) -> Result<String, String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err("タイトルは必須です".to_string());
    }
    if trimmed.chars().count() > 120 {
        return Err("タイトルは120文字以内で入力してください".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn validate_optional_date(
    value: Option<&str>,
    field_label: &str,
) -> Result<Option<String>, String> {
    let Some(raw_value) = value else {
        return Ok(None);
    };
    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Date::parse(trimmed, DATE_FORMAT)
        .map_err(|_| format!("{field_label}はYYYY-MM-DD形式で入力してください"))?;
    Ok(Some(trimmed.to_string()))
}

pub fn validate_date_range(
    planned_start_date: &Option<String>,
    due_date: &Option<String>,
) -> Result<(), String> {
    if let (Some(planned_start_date), Some(due_date)) = (planned_start_date, due_date) {
        if due_date < planned_start_date {
            return Err("期限日は開始予定日より前にできません".to_string());
        }
    }
    Ok(())
}

pub fn validate_optional_time(
    value: Option<&str>,
    field_label: &str,
) -> Result<Option<String>, String> {
    let Some(raw_value) = value else {
        return Ok(None);
    };
    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Time::parse(trimmed, TIME_FORMAT)
        .map_err(|_| format!("{field_label}はHH:MM形式で入力してください"))?;
    Ok(Some(trimmed.to_string()))
}

pub fn validate_due_time_requires_due_date(
    due_date: &Option<String>,
    due_time: &Option<String>,
) -> Result<(), String> {
    if due_date.is_none() && due_time.is_some() {
        return Err("期限時刻を設定する場合は期限日も設定してください".to_string());
    }
    Ok(())
}

pub fn validate_memo(value: Option<&str>) -> Result<String, String> {
    let memo = value.unwrap_or_default();
    if memo.chars().count() > MEMO_MAX_CHARS {
        return Err(format!("メモは{MEMO_MAX_CHARS}文字以内で入力してください"));
    }
    Ok(memo.to_string())
}

pub fn assert_timer_startable(status: &WorkStatus) -> Result<(), String> {
    match status {
        WorkStatus::Done | WorkStatus::Archived => {
            Err("完了済みまたはアーカイブ済みの対象はタイマーを開始できません".to_string())
        }
        WorkStatus::Todo | WorkStatus::InProgress => Ok(()),
    }
}

pub fn assert_completable(status: &WorkStatus) -> Result<(), String> {
    match status {
        WorkStatus::Archived => Err("アーカイブ済みの対象は完了できません".to_string()),
        WorkStatus::Todo | WorkStatus::InProgress | WorkStatus::Done => Ok(()),
    }
}
