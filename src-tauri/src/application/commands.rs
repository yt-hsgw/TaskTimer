use tauri::State;

type DatabaseState<'a> = State<'a, crate::infrastructure::sqlite::SqliteDatabase>;
type ClockState<'a> = State<'a, crate::infrastructure::clock::SystemClock>;
type NotificationGatewayState<'a> =
    State<'a, crate::infrastructure::notification::TauriLocalNotificationGateway>;
const TASK_LIST_LIMIT: i64 = 200;

#[tauri::command]
pub fn health_check(database: DatabaseState<'_>) -> Result<&'static str, String> {
    let _database_path = database.path();
    Ok("tauri-ready")
}

#[tauri::command]
pub fn list_calendar_items(
    database: DatabaseState<'_>,
    start_date: String,
    end_date: String,
) -> Result<Vec<super::dto::WeekCalendarItemDto>, String> {
    use crate::application::repositories::CalendarRepository;

    database
        .list_calendar_items(&start_date, &end_date)
        .map(|items| items.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn list_week_calendar_items(
    database: DatabaseState<'_>,
    week_start_date: String,
) -> Result<Vec<super::dto::WeekCalendarItemDto>, String> {
    use crate::application::repositories::CalendarRepository;

    database
        .list_week_calendar_items(&week_start_date)
        .map(|items| items.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn list_tasks(
    database: DatabaseState<'_>,
) -> Result<Vec<super::dto::TaskWithSubtasksDto>, String> {
    use crate::application::repositories::TaskReadRepository;

    database
        .list_tasks_with_subtasks(TASK_LIST_LIMIT)
        .map(|tasks| tasks.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn list_task_lists(
    database: DatabaseState<'_>,
) -> Result<Vec<super::dto::TaskListDto>, String> {
    use crate::application::repositories::TaskReadRepository;

    database
        .list_task_lists()
        .map(|lists| lists.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn list_task_rows(
    database: DatabaseState<'_>,
    list_id: Option<String>,
) -> Result<Vec<super::dto::TaskRowDto>, String> {
    use crate::application::repositories::TaskReadRepository;

    database
        .list_task_rows(list_id.as_deref(), TASK_LIST_LIMIT)
        .map(|rows| rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn list_archived_task_rows(
    database: DatabaseState<'_>,
) -> Result<Vec<super::dto::TaskRowDto>, String> {
    use crate::application::repositories::TaskReadRepository;

    database
        .list_archived_task_rows(TASK_LIST_LIMIT)
        .map(|rows| rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn get_active_timer(
    database: DatabaseState<'_>,
) -> Result<Option<super::dto::ActiveTimerDto>, String> {
    use crate::application::repositories::TimerRepository;

    database
        .get_active_timer()
        .map(|active_timer| active_timer.map(Into::into))
}

#[tauri::command]
pub fn get_notification_display_mode(database: DatabaseState<'_>) -> Result<String, String> {
    use crate::application::repositories::NotificationPreferenceRepository;

    database
        .get_notification_display_mode()
        .map(|display_mode| display_mode.as_str().to_string())
}

#[tauri::command]
pub fn get_notifications_enabled(database: DatabaseState<'_>) -> Result<bool, String> {
    use crate::application::repositories::NotificationPreferenceRepository;

    database.get_notifications_enabled()
}

#[tauri::command]
pub fn list_notification_failure_history(
    database: DatabaseState<'_>,
) -> Result<Vec<super::dto::NotificationDeliveryAttemptDto>, String> {
    super::usecases::list_notification_failure_history(database.inner())
        .map(|attempts| attempts.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn create_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::create_task(database.inner(), clock.inner(), request.into()).map(Into::into)
}

#[tauri::command]
pub fn create_task_list(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateTaskListRequestDto,
) -> Result<super::dto::TaskListDto, String> {
    super::usecases::create_task_list(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn update_task_list(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateTaskListRequestDto,
) -> Result<super::dto::TaskListDto, String> {
    let list_id = request.list_id.clone();
    super::usecases::update_task_list(database.inner(), clock.inner(), list_id, request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn delete_task_list(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::DeleteTaskListRequestDto,
) -> Result<(), String> {
    super::usecases::delete_task_list(database.inner(), clock.inner(), request.list_id)
}

#[tauri::command]
pub fn create_subtask(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateSubtaskRequestDto,
) -> Result<super::dto::SubtaskDto, String> {
    let task_id = request.task_id.clone();
    super::usecases::create_subtask(database.inner(), clock.inner(), task_id, request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn update_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    let task_id = request.task_id.clone();
    super::usecases::update_task(database.inner(), clock.inner(), task_id, request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn update_subtask(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateSubtaskRequestDto,
) -> Result<super::dto::SubtaskDto, String> {
    let subtask_id = request.subtask_id.clone();
    super::usecases::update_subtask(database.inner(), clock.inner(), subtask_id, request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn start_timer(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::StartTimerRequestDto,
) -> Result<super::dto::ActiveTimerDto, String> {
    super::usecases::start_timer(database.inner(), clock.inner(), request.target.try_into()?)
        .map(Into::into)
}

#[tauri::command]
pub fn pause_active_timer(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActiveTimerDto, String> {
    super::usecases::pause_active_timer(database.inner(), clock.inner()).map(Into::into)
}

#[tauri::command]
pub fn resume_active_timer(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActiveTimerDto, String> {
    super::usecases::resume_active_timer(database.inner(), clock.inner()).map(Into::into)
}

#[tauri::command]
pub fn stop_active_timer(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActiveTimerDto, String> {
    super::usecases::stop_active_timer(database.inner(), clock.inner()).map(Into::into)
}

#[tauri::command]
pub fn complete_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CompleteTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::complete_task(
        database.inner(),
        clock.inner(),
        request.task_id,
        request.allow_incomplete_subtasks,
    )
    .map(Into::into)
}

#[tauri::command]
pub fn reopen_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::ReopenTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::reopen_task(database.inner(), clock.inner(), request.task_id).map(Into::into)
}

#[tauri::command]
pub fn complete_subtask(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CompleteSubtaskRequestDto,
) -> Result<super::dto::SubtaskDto, String> {
    super::usecases::complete_subtask(database.inner(), clock.inner(), request.subtask_id)
        .map(Into::into)
}

#[tauri::command]
pub fn reopen_subtask(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::ReopenSubtaskRequestDto,
) -> Result<super::dto::SubtaskDto, String> {
    super::usecases::reopen_subtask(database.inner(), clock.inner(), request.subtask_id)
        .map(Into::into)
}

#[tauri::command]
pub fn toggle_task_favorite(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::ToggleTaskFavoriteRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::toggle_task_favorite(
        database.inner(),
        clock.inner(),
        request.task_id,
        request.is_favorite,
    )
    .map(Into::into)
}

#[tauri::command]
pub fn archive_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::ArchiveTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::archive_task(database.inner(), clock.inner(), request.task_id).map(Into::into)
}

#[tauri::command]
pub fn restore_archived_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::RestoreArchivedTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::restore_archived_task(database.inner(), clock.inner(), request.task_id)
        .map(Into::into)
}

#[tauri::command]
pub fn delete_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::DeleteTaskRequestDto,
) -> Result<(), String> {
    super::usecases::delete_task(database.inner(), clock.inner(), request.task_id)
}

#[tauri::command]
pub fn delete_subtask(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::DeleteSubtaskRequestDto,
) -> Result<(), String> {
    super::usecases::delete_subtask(database.inner(), clock.inner(), request.subtask_id)
}

#[tauri::command]
pub fn update_notification_display_mode(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateNotificationDisplayModeRequestDto,
) -> Result<String, String> {
    let display_mode = request.try_into()?;
    super::usecases::update_notification_display_mode(database.inner(), clock.inner(), display_mode)
        .map(|display_mode| display_mode.as_str().to_string())
}

#[tauri::command]
pub fn update_notifications_enabled(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateNotificationsEnabledRequestDto,
) -> Result<bool, String> {
    super::usecases::update_notifications_enabled(database.inner(), clock.inner(), request.enabled)
}

#[tauri::command]
pub fn dispatch_due_notifications(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    notification_gateway: NotificationGatewayState<'_>,
) -> Result<super::dto::NotificationDispatchSummaryDto, String> {
    super::usecases::dispatch_due_notifications(
        database.inner(),
        notification_gateway.inner(),
        clock.inner(),
    )
    .map(Into::into)
}

#[tauri::command]
pub fn create_sqlite_backup(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateSqliteBackupRequestDto,
) -> Result<super::dto::SqliteBackupDto, String> {
    super::usecases::create_sqlite_backup(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn restore_sqlite_backup(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::RestoreSqliteBackupRequestDto,
) -> Result<super::dto::SqliteRestoreDto, String> {
    super::usecases::restore_sqlite_backup(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn create_json_export(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateDataExportRequestDto,
) -> Result<super::dto::DataExportDto, String> {
    super::usecases::create_json_export(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn create_csv_export(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateDataExportRequestDto,
) -> Result<super::dto::DataExportDto, String> {
    super::usecases::create_csv_export(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}
