mod application;
mod domain;
mod infrastructure;

use application::commands::{
    get_active_timer, get_notification_display_mode, health_check, list_week_calendar_items,
};
use infrastructure::sqlite::SqliteDatabase;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database = SqliteDatabase::open(app.handle())
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            list_week_calendar_items,
            get_active_timer,
            get_notification_display_mode
        ])
        .run(tauri::generate_context!())
        .expect("TaskTimerの起動に失敗しました");
}
