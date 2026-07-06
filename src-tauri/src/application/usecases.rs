use crate::domain::{
    task::{validate_date_range, validate_memo, validate_optional_date, validate_title},
    timer::WorkTargetRef,
};

use super::{
    clock::Clock,
    repositories::{
        ActiveTimer, RepositoryResult, SubtaskRecord, TaskRecord, TaskTimerCommandRepository,
        WorkItemCreate,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemDraft {
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub memo: Option<String>,
}

pub fn create_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    draft: WorkItemDraft,
) -> RepositoryResult<TaskRecord> {
    repository.create_task(validate_work_item_draft(draft, clock.now_utc_iso8601())?)
}

pub fn create_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
    draft: WorkItemDraft,
) -> RepositoryResult<SubtaskRecord> {
    let task_id = validate_identifier(&task_id, "親タスクID")?;
    repository.create_subtask(
        task_id,
        validate_work_item_draft(draft, clock.now_utc_iso8601())?,
    )
}

pub fn start_timer(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    target: WorkTargetRef,
) -> RepositoryResult<ActiveTimer> {
    let target = WorkTargetRef {
        target_type: target.target_type,
        id: validate_identifier(&target.id, "対象ID")?,
    };
    repository.start_timer(target, clock.now_utc_iso8601())
}

pub fn stop_active_timer(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActiveTimer> {
    repository.stop_active_timer(clock.now_utc_iso8601())
}

fn validate_work_item_draft(draft: WorkItemDraft, now: String) -> RepositoryResult<WorkItemCreate> {
    let title = validate_title(&draft.title)?;
    let planned_start_date = validate_optional_date(draft.planned_start_date.as_deref(), "開始日")?;
    let due_date = validate_optional_date(draft.due_date.as_deref(), "終了日")?;
    validate_date_range(&planned_start_date, &due_date)?;
    let memo = validate_memo(draft.memo.as_deref())?;

    Ok(WorkItemCreate {
        title,
        planned_start_date,
        due_date,
        memo,
        now,
    })
}

fn validate_identifier(value: &str, field_label: &str) -> RepositoryResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_label}は必須です"));
    }
    if trimmed.chars().count() > 128 {
        return Err(format!("{field_label}は128文字以内で入力してください"));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_work_item_draft_rejects_blank_title() {
        let result = validate_work_item_draft(
            WorkItemDraft {
                title: "   ".to_string(),
                planned_start_date: None,
                due_date: None,
                memo: None,
            },
            "2026-07-06T00:00:00Z".to_string(),
        );

        assert!(result.expect_err("blank title").contains("タイトル"));
    }

    #[test]
    fn validate_work_item_draft_rejects_reversed_date_range() {
        let result = validate_work_item_draft(
            WorkItemDraft {
                title: "設計レビュー".to_string(),
                planned_start_date: Some("2026-07-07".to_string()),
                due_date: Some("2026-07-06".to_string()),
                memo: None,
            },
            "2026-07-06T00:00:00Z".to_string(),
        );

        assert!(result.expect_err("reversed date range").contains("期限日"));
    }
}
