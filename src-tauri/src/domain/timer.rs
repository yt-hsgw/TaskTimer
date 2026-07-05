#![allow(dead_code)]

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
pub struct WorkTargetRef {
    pub target_type: WorkTargetType,
    pub id: String,
}
