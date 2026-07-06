mod application;
mod domain;
mod infrastructure;

use application::commands::{
    complete_subtask, complete_task, create_subtask, create_task, delete_subtask, delete_task,
    dispatch_due_notifications, get_active_timer, get_notification_display_mode, health_check,
    list_tasks, list_week_calendar_items, start_timer, stop_active_timer,
    update_notification_display_mode,
};
use infrastructure::{
    clock::SystemClock, notification::TauriLocalNotificationGateway, sqlite::SqliteDatabase,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let database = SqliteDatabase::open(app.handle()).map_err(std::io::Error::other)?;
            app.manage(database);
            app.manage(SystemClock);
            app.manage(TauriLocalNotificationGateway::new(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            list_tasks,
            list_week_calendar_items,
            get_active_timer,
            get_notification_display_mode,
            create_task,
            create_subtask,
            start_timer,
            stop_active_timer,
            complete_task,
            complete_subtask,
            delete_task,
            delete_subtask,
            update_notification_display_mode,
            dispatch_due_notifications
        ])
        .run(tauri::generate_context!())
        .expect("TaskTimerの起動に失敗しました");
}
