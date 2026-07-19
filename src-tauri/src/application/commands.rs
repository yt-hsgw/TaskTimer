use tauri::State;

type DatabaseState<'a> = State<'a, crate::infrastructure::sqlite::SqliteDatabase>;
type ClockState<'a> = State<'a, crate::infrastructure::clock::SystemClock>;
type NotificationGatewayState<'a> =
    State<'a, crate::infrastructure::notification::TauriLocalNotificationGateway>;
type NativeNotificationGatewayState<'a> =
    State<'a, crate::infrastructure::notification::TauriNativeNotificationRegistrationGateway>;
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
pub fn list_task_page(
    database: DatabaseState<'_>,
    request: super::dto::ListTaskPageRequestDto,
) -> Result<super::dto::TaskPageDto, String> {
    super::usecases::list_task_page(database.inner(), request.into()).map(Into::into)
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
pub fn list_board_columns(
    database: DatabaseState<'_>,
) -> Result<Vec<super::dto::BoardColumnDto>, String> {
    super::usecases::list_board_columns(database.inner())
        .map(|columns| columns.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn list_tags(database: DatabaseState<'_>) -> Result<Vec<super::dto::TagDto>, String> {
    super::usecases::list_tags(database.inner())
        .map(|tags| tags.into_iter().map(Into::into).collect())
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
pub fn get_task_timer_settings(
    database: DatabaseState<'_>,
) -> Result<super::dto::TaskTimerSettingsDto, String> {
    super::usecases::get_task_timer_settings(database.inner()).map(Into::into)
}

#[tauri::command]
pub fn update_task_timer_settings(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateTaskTimerSettingsRequestDto,
) -> Result<super::dto::TaskTimerSettingsDto, String> {
    super::usecases::update_task_timer_settings(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn sync_expired_task_countdown(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    notification_gateway: NotificationGatewayState<'_>,
) -> Result<super::dto::TaskCountdownExpirySyncDto, String> {
    super::usecases::sync_expired_task_countdown(
        database.inner(),
        notification_gateway.inner(),
        clock.inner(),
    )
    .map(Into::into)
}

#[tauri::command]
pub fn get_pomodoro_settings(
    database: DatabaseState<'_>,
) -> Result<super::dto::PomodoroSettingsDto, String> {
    super::usecases::get_pomodoro_settings(database.inner()).map(Into::into)
}

#[tauri::command]
pub fn update_pomodoro_settings(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdatePomodoroSettingsRequestDto,
) -> Result<super::dto::PomodoroSettingsDto, String> {
    super::usecases::update_pomodoro_settings(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn get_active_pomodoro(
    database: DatabaseState<'_>,
) -> Result<Option<super::dto::ActivePomodoroDto>, String> {
    super::usecases::get_active_pomodoro(database.inner())
        .map(|active_pomodoro| active_pomodoro.map(Into::into))
}

#[tauri::command]
pub fn sync_expired_pomodoro(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    notification_gateway: NotificationGatewayState<'_>,
) -> Result<super::dto::PomodoroExpirySyncDto, String> {
    super::usecases::sync_expired_pomodoro(
        database.inner(),
        notification_gateway.inner(),
        clock.inner(),
    )
    .map(Into::into)
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
pub fn get_next_pending_notification(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<Option<super::dto::NextNotificationScheduleDto>, String> {
    super::usecases::get_next_pending_notification(database.inner(), clock.inner())
        .map(|schedule| schedule.map(Into::into))
}

#[tauri::command]
pub fn sync_notifications(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    notification_gateway: NotificationGatewayState<'_>,
) -> Result<super::dto::NotificationSyncResultDto, String> {
    super::usecases::sync_notifications(
        database.inner(),
        notification_gateway.inner(),
        clock.inner(),
    )
    .map(Into::into)
}

#[tauri::command]
pub fn process_notification_os_registrations(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    native_notification_gateway: NativeNotificationGatewayState<'_>,
) -> Result<super::dto::NativeNotificationRegistrationSummaryDto, String> {
    super::usecases::process_notification_os_registration_jobs(
        database.inner(),
        native_notification_gateway.inner(),
        clock.inner(),
    )
    .map(Into::into)
}

#[tauri::command]
pub fn get_ui_preferences(
    database: DatabaseState<'_>,
) -> Result<super::dto::UiPreferencesDto, String> {
    super::usecases::get_ui_preferences(database.inner()).map(Into::into)
}

#[tauri::command]
pub fn update_ui_preferences(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateUiPreferencesRequestDto,
) -> Result<super::dto::UiPreferencesDto, String> {
    super::usecases::update_ui_preferences(database.inner(), clock.inner(), request.into())
        .map(Into::into)
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
pub fn create_scheduled_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateScheduledTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    let (draft, schedule) = request.into();
    super::usecases::create_scheduled_task(database.inner(), clock.inner(), draft, schedule)
        .map(Into::into)
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
pub fn create_board_column(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateBoardColumnRequestDto,
) -> Result<super::dto::BoardColumnDto, String> {
    super::usecases::create_board_column(database.inner(), clock.inner(), request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn update_board_column(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateBoardColumnRequestDto,
) -> Result<super::dto::BoardColumnDto, String> {
    let column_id = request.column_id.clone();
    super::usecases::update_board_column(database.inner(), clock.inner(), column_id, request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn reorder_board_columns(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::ReorderBoardColumnsRequestDto,
) -> Result<Vec<super::dto::BoardColumnDto>, String> {
    super::usecases::reorder_board_columns(
        database.inner(),
        clock.inner(),
        request.ordered_column_ids,
    )
    .map(|columns| columns.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn delete_board_column(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::DeleteBoardColumnRequestDto,
) -> Result<(), String> {
    super::usecases::delete_board_column(
        database.inner(),
        clock.inner(),
        request.column_id,
        request.move_tasks_to_column_id,
    )
}

#[tauri::command]
pub fn move_task_to_board_column(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::MoveTaskToBoardColumnRequestDto,
) -> Result<(), String> {
    super::usecases::move_task_to_board_column(
        database.inner(),
        clock.inner(),
        request.task_id,
        request.board_column_id,
    )
}

#[tauri::command]
pub fn create_tag(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateTagRequestDto,
) -> Result<super::dto::TagDto, String> {
    super::usecases::create_tag(database.inner(), clock.inner(), request.into()).map(Into::into)
}

#[tauri::command]
pub fn update_tag(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateTagRequestDto,
) -> Result<super::dto::TagDto, String> {
    let tag_id = request.tag_id.clone();
    super::usecases::update_tag(database.inner(), clock.inner(), tag_id, request.into())
        .map(Into::into)
}

#[tauri::command]
pub fn delete_tag(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::DeleteTagRequestDto,
) -> Result<(), String> {
    super::usecases::delete_tag(database.inner(), clock.inner(), request.tag_id)
}

#[tauri::command]
pub fn attach_tag_to_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::AttachTaskTagRequestDto,
) -> Result<super::dto::TaskTagDto, String> {
    super::usecases::attach_tag_to_task(
        database.inner(),
        clock.inner(),
        request.task_id,
        request.tag_id,
    )
    .map(Into::into)
}

#[tauri::command]
pub fn detach_tag_from_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::DetachTaskTagRequestDto,
) -> Result<(), String> {
    super::usecases::detach_tag_from_task(
        database.inner(),
        clock.inner(),
        request.task_id,
        request.tag_id,
    )
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
pub fn resize_scheduled_work_item(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::ResizeScheduledWorkItemRequestDto,
) -> Result<(), String> {
    super::usecases::resize_scheduled_work_item(
        database.inner(),
        clock.inner(),
        request.target.try_into()?,
        request.schedule.into(),
    )
}

#[tauri::command]
pub fn move_scheduled_work_item(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::MoveScheduledWorkItemRequestDto,
) -> Result<(), String> {
    super::usecases::move_scheduled_work_item(
        database.inner(),
        clock.inner(),
        request.target.try_into()?,
        request.destination.into(),
    )
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
pub fn start_pomodoro(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::StartPomodoroRequestDto,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::start_pomodoro(database.inner(), clock.inner(), request.target.try_into()?)
        .map(Into::into)
}

#[tauri::command]
pub fn pause_pomodoro(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::pause_pomodoro(database.inner(), clock.inner()).map(Into::into)
}

#[tauri::command]
pub fn resume_pomodoro(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::resume_pomodoro(database.inner(), clock.inner()).map(Into::into)
}

#[tauri::command]
pub fn complete_pomodoro_work_phase(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::complete_pomodoro_work_phase(database.inner(), clock.inner()).map(Into::into)
}

#[tauri::command]
pub fn start_pomodoro_break(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::PomodoroSessionRequestDto,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::start_pomodoro_break(
        database.inner(),
        clock.inner(),
        request.pomodoro_session_id,
    )
    .map(Into::into)
}

#[tauri::command]
pub fn skip_pomodoro_break(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::PomodoroSessionRequestDto,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::skip_pomodoro_break(
        database.inner(),
        clock.inner(),
        request.pomodoro_session_id,
    )
    .map(Into::into)
}

#[tauri::command]
pub fn complete_pomodoro_break(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::complete_pomodoro_break(database.inner(), clock.inner()).map(Into::into)
}

#[tauri::command]
pub fn cancel_pomodoro(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActivePomodoroDto, String> {
    super::usecases::cancel_pomodoro(database.inner(), clock.inner()).map(Into::into)
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
pub fn update_task_status(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::UpdateTaskStatusRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::update_task_status(
        database.inner(),
        clock.inner(),
        request.task_id,
        request.status,
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
