use crate::domain::{
    notification::{build_notification_content, NotificationDisplayMode},
    task::{validate_date_range, validate_memo, validate_optional_date, validate_title},
    timer::WorkTargetRef,
};

use super::{
    clock::Clock,
    notification::{LocalNotificationGateway, LocalNotificationMessage},
    repositories::{
        ActiveTimer, NotificationCommandRepository, NotificationDispatchSummary,
        NotificationPreferenceRepository, RepositoryResult, SubtaskRecord, TaskRecord,
        TaskTimerCommandRepository, WorkItemCreate,
    },
};

const NOTIFICATION_DISPATCH_LIMIT: i64 = 20;

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

pub fn complete_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
    allow_incomplete_subtasks: bool,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.complete_task(task_id, allow_incomplete_subtasks, clock.now_utc_iso8601())
}

pub fn complete_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    subtask_id: String,
) -> RepositoryResult<SubtaskRecord> {
    let subtask_id = validate_identifier(&subtask_id, "サブタスクID")?;
    repository.complete_subtask(subtask_id, clock.now_utc_iso8601())
}

pub fn delete_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
) -> RepositoryResult<()> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.delete_task(task_id, clock.now_utc_iso8601())
}

pub fn delete_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    subtask_id: String,
) -> RepositoryResult<()> {
    let subtask_id = validate_identifier(&subtask_id, "サブタスクID")?;
    repository.delete_subtask(subtask_id, clock.now_utc_iso8601())
}

pub fn update_notification_display_mode(
    repository: &impl NotificationCommandRepository,
    clock: &impl Clock,
    display_mode: NotificationDisplayMode,
) -> RepositoryResult<NotificationDisplayMode> {
    repository.update_notification_display_mode(display_mode, clock.now_utc_iso8601())
}

pub fn dispatch_due_notifications(
    repository: &(impl NotificationCommandRepository + NotificationPreferenceRepository),
    notification_gateway: &impl LocalNotificationGateway,
    clock: &impl Clock,
) -> RepositoryResult<NotificationDispatchSummary> {
    let now = clock.now_utc_iso8601();
    let display_mode = repository.get_notification_display_mode()?;
    let jobs = repository.list_due_notification_jobs(&now, NOTIFICATION_DISPATCH_LIMIT)?;

    let mut summary = NotificationDispatchSummary {
        attempted: jobs.len(),
        succeeded: 0,
        failed: 0,
        last_error: None,
    };

    for job in jobs {
        let content = build_notification_content(&display_mode, &job.target_title);
        let result = notification_gateway.send(&LocalNotificationMessage {
            title: content.title,
            body: content.body,
        });

        match result {
            Ok(()) => {
                repository.mark_notification_registered(&job.id, &now)?;
                summary.succeeded += 1;
            }
            Err(error) => {
                repository.mark_notification_failed(&job.id, &error, &now)?;
                summary.failed += 1;
                summary.last_error = Some(error);
            }
        }
    }

    Ok(summary)
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
