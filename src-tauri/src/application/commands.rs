use tauri::State;

type DatabaseState<'a> = State<'a, crate::infrastructure::sqlite::SqliteDatabase>;
type ClockState<'a> = State<'a, crate::infrastructure::clock::SystemClock>;
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
pub fn start_timer(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
    request: super::dto::StartTimerRequestDto,
) -> Result<super::dto::ActiveTimerDto, String> {
    super::usecases::start_timer(database.inner(), clock.inner(), request.target.try_into()?)
        .map(Into::into)
}

#[tauri::command]
pub fn stop_active_timer(
    database: DatabaseState<'_>,
    clock: ClockState<'_>,
) -> Result<super::dto::ActiveTimerDto, String> {
    super::usecases::stop_active_timer(database.inner(), clock.inner()).map(Into::into)
}
