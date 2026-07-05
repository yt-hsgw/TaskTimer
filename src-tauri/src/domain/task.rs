#![allow(dead_code)]

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

pub fn assert_timer_startable(status: &WorkStatus) -> Result<(), String> {
    match status {
        WorkStatus::Done | WorkStatus::Archived => {
            Err("完了済みまたはアーカイブ済みの対象はタイマーを開始できません".to_string())
        }
        WorkStatus::Todo | WorkStatus::InProgress => Ok(()),
    }
}
