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
pub fn create_task(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CreateTaskRequestDto,
) -> Result<super::dto::TaskDto, String> {
    super::usecases::create_task(database.inner(), clock.inner(), request.into()).map(Into::into)
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
pub fn complete_subtask(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::CompleteSubtaskRequestDto,
) -> Result<super::dto::SubtaskDto, String> {
    super::usecases::complete_subtask(database.inner(), clock.inner(), request.subtask_id)
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
