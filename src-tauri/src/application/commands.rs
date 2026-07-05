use tauri::State;

#[tauri::command]
pub fn health_check(
    database: State<'_, crate::infrastructure::sqlite::SqliteDatabase>,
) -> Result<&'static str, String> {
    let _database_path = database.path();
    Ok("tauri-ready")
}

#[tauri::command]
pub fn list_week_calendar_items(
    database: State<'_, crate::infrastructure::sqlite::SqliteDatabase>,
    week_start_date: String,
) -> Result<Vec<super::dto::WeekCalendarItemDto>, String> {
    use crate::application::repositories::CalendarRepository;

    database
        .list_week_calendar_items(&week_start_date)
        .map(|items| items.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn get_active_timer(
    database: State<'_, crate::infrastructure::sqlite::SqliteDatabase>,
) -> Result<Option<super::dto::ActiveTimerDto>, String> {
    use crate::application::repositories::TimerRepository;

    database
        .get_active_timer()
        .map(|active_timer| active_timer.map(Into::into))
}

#[tauri::command]
pub fn get_notification_display_mode(
    database: State<'_, crate::infrastructure::sqlite::SqliteDatabase>,
) -> Result<String, String> {
    use crate::application::repositories::NotificationPreferenceRepository;

    database
        .get_notification_display_mode()
        .map(|display_mode| display_mode.as_str().to_string())
}
