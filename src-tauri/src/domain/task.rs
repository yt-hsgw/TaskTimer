#![allow(dead_code)]

use time::{macros::format_description, Date, PrimitiveDateTime, Time};

const DATE_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");
const TIME_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[hour]:[minute]");
const MEMO_MAX_CHARS: usize = 10_000;
const TASK_LIST_NAME_MAX_CHARS: usize = 80;
const TAG_NAME_MAX_CHARS: usize = 40;
const BOARD_COLUMN_NAME_MAX_CHARS: usize = 80;
const SCHEDULE_MAX_DAYS: i64 = 366;
const SCHEDULE_MINUTE_STEP: u8 = 15;

pub const DEFAULT_TASK_LIST_ID: &str = "default";
pub const DEFAULT_TASK_LIST_NAME: &str = "タスク";
pub const DEFAULT_TASK_LIST_COLOR_TOKEN: &str = "green";
pub const TASK_LIST_COLOR_TOKENS: &[&str] = &["green", "blue", "amber", "rose", "violet", "gray"];
pub const DEFAULT_BOARD_COLUMN_ID: &str = "board-todo";
pub const IN_PROGRESS_BOARD_COLUMN_ID: &str = "board-in-progress";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkSchedule {
    pub start_date: String,
    pub start_time: Option<String>,
    pub end_date: String,
    pub end_time: Option<String>,
    pub is_all_day: bool,
}

impl WorkSchedule {
    pub fn parse(
        start_date: &str,
        start_time: Option<&str>,
        end_date: &str,
        end_time: Option<&str>,
        is_all_day: bool,
    ) -> Result<Self, String> {
        let parsed_start_date = parse_schedule_date(start_date, "予定開始日")?;
        let parsed_end_date = parse_schedule_date(end_date, "予定終了日")?;

        if is_all_day {
            if start_time.is_some() || end_time.is_some() {
                return Err("終日予定には開始時刻と終了時刻を設定できません".to_string());
            }
            let days = (parsed_end_date - parsed_start_date).whole_days();
            if !(0..SCHEDULE_MAX_DAYS).contains(&days) {
                return Err("終日予定は開始日から366日以内で設定してください".to_string());
            }
            return Ok(Self {
                start_date: start_date.to_string(),
                start_time: None,
                end_date: end_date.to_string(),
                end_time: None,
                is_all_day: true,
            });
        }

        let start_time = start_time.ok_or_else(|| "予定開始時刻は必須です".to_string())?;
        let end_time = end_time.ok_or_else(|| "予定終了時刻は必須です".to_string())?;
        let parsed_start_time = parse_schedule_time(start_time, "予定開始時刻")?;
        let parsed_end_time = parse_schedule_time(end_time, "予定終了時刻")?;
        validate_schedule_minute_step(parsed_start_time, "予定開始時刻")?;
        validate_schedule_minute_step(parsed_end_time, "予定終了時刻")?;

        let start = PrimitiveDateTime::new(parsed_start_date, parsed_start_time);
        let end = PrimitiveDateTime::new(parsed_end_date, parsed_end_time);
        if end <= start {
            return Err("予定終了日時は予定開始日時より後にしてください".to_string());
        }
        if end - start > time::Duration::days(SCHEDULE_MAX_DAYS) {
            return Err("予定期間は366日以内で設定してください".to_string());
        }

        Ok(Self {
            start_date: start_date.to_string(),
            start_time: Some(start_time.to_string()),
            end_date: end_date.to_string(),
            end_time: Some(end_time.to_string()),
            is_all_day: false,
        })
    }
}

fn parse_schedule_date(value: &str, field_label: &str) -> Result<Date, String> {
    Date::parse(value, DATE_FORMAT)
        .map_err(|_| format!("{field_label}はYYYY-MM-DD形式で入力してください"))
}

fn parse_schedule_time(value: &str, field_label: &str) -> Result<Time, String> {
    Time::parse(value, TIME_FORMAT)
        .map_err(|_| format!("{field_label}はHH:mm形式で入力してください"))
}

fn validate_schedule_minute_step(value: Time, field_label: &str) -> Result<(), String> {
    if !value.minute().is_multiple_of(SCHEDULE_MINUTE_STEP) {
        return Err(format!("{field_label}は15分単位で入力してください"));
    }
    Ok(())
}

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

pub fn validate_task_list_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("リスト名は必須です".to_string());
    }
    if trimmed.chars().count() > TASK_LIST_NAME_MAX_CHARS {
        return Err(format!(
            "リスト名は{TASK_LIST_NAME_MAX_CHARS}文字以内で入力してください"
        ));
    }
    Ok(trimmed.to_string())
}

pub fn validate_task_list_color_token(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if TASK_LIST_COLOR_TOKENS.contains(&trimmed) {
        return Ok(trimmed.to_string());
    }

    Err("リスト色は許可済みの色から選択してください".to_string())
}

pub fn validate_tag_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("タグ名は必須です".to_string());
    }
    if trimmed.chars().count() > TAG_NAME_MAX_CHARS {
        return Err(format!(
            "タグ名は{TAG_NAME_MAX_CHARS}文字以内で入力してください"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err("タグ名に制御文字は使用できません".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn validate_board_column_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("状態名は必須です".to_string());
    }
    if trimmed.chars().count() > BOARD_COLUMN_NAME_MAX_CHARS {
        return Err(format!(
            "状態名は{BOARD_COLUMN_NAME_MAX_CHARS}文字以内で入力してください"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err("状態名に制御文字は使用できません".to_string());
    }
    Ok(trimmed.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_schedule_accepts_timed_and_all_day_ranges() {
        let timed = WorkSchedule::parse(
            "2026-07-20",
            Some("09:15"),
            "2026-07-20",
            Some("10:30"),
            false,
        )
        .expect("timed schedule");
        assert_eq!(timed.end_time.as_deref(), Some("10:30"));

        let all_day = WorkSchedule::parse("2026-07-20", None, "2026-07-22", None, true)
            .expect("all-day schedule");
        assert!(all_day.is_all_day);
    }

    #[test]
    fn work_schedule_rejects_reversed_and_non_quarter_hour_ranges() {
        let reversed = WorkSchedule::parse(
            "2026-07-20",
            Some("10:00"),
            "2026-07-20",
            Some("09:45"),
            false,
        );
        assert!(reversed.expect_err("reversed range").contains("後"));

        let off_step = WorkSchedule::parse(
            "2026-07-20",
            Some("09:10"),
            "2026-07-20",
            Some("10:00"),
            false,
        );
        assert!(off_step.expect_err("minute step").contains("15分"));
    }

    #[test]
    fn work_schedule_rejects_times_on_all_day_range() {
        let result = WorkSchedule::parse("2026-07-20", Some("00:00"), "2026-07-20", None, true);
        assert!(result.expect_err("all day time").contains("終日"));
    }
}
