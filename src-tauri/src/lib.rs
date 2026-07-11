mod application;
mod domain;
mod infrastructure;

use application::commands::{
    complete_subtask, complete_task, create_subtask, create_task, delete_subtask, delete_task,
    dispatch_due_notifications, get_active_timer, get_notification_display_mode, health_check,
    list_task_lists, list_task_rows, list_tasks, list_week_calendar_items, pause_active_timer,
    reopen_task, resume_active_timer, start_timer, stop_active_timer, toggle_task_favorite,
    update_notification_display_mode, update_subtask, update_task,
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
            list_task_lists,
            list_task_rows,
            list_week_calendar_items,
            get_active_timer,
            get_notification_display_mode,
            create_task,
            create_subtask,
            update_task,
            update_subtask,
            start_timer,
            pause_active_timer,
            resume_active_timer,
            stop_active_timer,
            complete_task,
            reopen_task,
            complete_subtask,
            toggle_task_favorite,
            delete_task,
            delete_subtask,
            update_notification_display_mode,
            dispatch_due_notifications
        ])
        .run(tauri::generate_context!())
        .expect("TaskTimerの起動に失敗しました");
}
