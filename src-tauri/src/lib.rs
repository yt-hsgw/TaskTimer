mod application;
mod domain;
mod infrastructure;

use application::commands::{
    archive_task, assign_work_schedule, attach_tag_to_task, cancel_pomodoro,
    complete_pomodoro_break, complete_pomodoro_work_phase, complete_subtask, complete_task,
    create_board_column, create_csv_export, create_json_export, create_scheduled_task,
    create_sqlite_backup, create_subtask, create_tag, create_task, create_task_in_board_column,
    create_task_list, delete_board_column, delete_completed_tasks_in_board_column, delete_subtask,
    delete_tag, delete_task, delete_task_list, detach_tag_from_task, dispatch_due_notifications,
    get_active_pomodoro, get_active_timer, get_next_pending_notification,
    get_notification_display_mode, get_notifications_enabled, get_pomodoro_settings,
    get_task_detail, get_task_timer_settings, get_ui_preferences, health_check,
    list_archived_task_rows, list_board_columns, list_calendar_items,
    list_notification_failure_history, list_tags, list_task_lists, list_task_page, list_task_rows,
    list_tasks, list_week_calendar_items, move_scheduled_work_item, move_task_to_board_column,
    pause_active_timer, pause_pomodoro, process_notification_os_registrations, reopen_subtask,
    reopen_task, reorder_board_columns, reorder_task_within_list, resize_scheduled_work_item,
    restore_archived_task, restore_sqlite_backup, resume_active_timer, resume_pomodoro,
    search_work_items, skip_pomodoro_break, start_pomodoro_break, start_standalone_pomodoro,
    start_timer, stop_active_timer, sync_expired_pomodoro, sync_expired_task_countdown,
    sync_notifications, toggle_task_favorite, update_board_column,
    update_notification_display_mode, update_notifications_enabled, update_pomodoro_settings,
    update_subtask, update_tag, update_task, update_task_list, update_task_status,
    update_task_timer_settings, update_ui_preferences,
};
use infrastructure::{
    clock::SystemClock,
    notification::{TauriLocalNotificationGateway, TauriNativeNotificationRegistrationGateway},
    sqlite::SqliteDatabase,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let database = SqliteDatabase::open(app.handle()).map_err(std::io::Error::other)?;
            app.manage(database);
            app.manage(SystemClock);
            app.manage(TauriLocalNotificationGateway::new(app.handle().clone()));
            app.manage(TauriNativeNotificationRegistrationGateway::new(
                app.handle().clone(),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            list_tasks,
            list_task_page,
            get_task_detail,
            search_work_items,
            list_task_lists,
            list_board_columns,
            list_tags,
            list_task_rows,
            list_archived_task_rows,
            list_calendar_items,
            list_week_calendar_items,
            get_active_timer,
            get_task_timer_settings,
            update_task_timer_settings,
            sync_expired_task_countdown,
            get_pomodoro_settings,
            update_pomodoro_settings,
            get_active_pomodoro,
            sync_expired_pomodoro,
            get_notification_display_mode,
            get_notifications_enabled,
            get_next_pending_notification,
            sync_notifications,
            process_notification_os_registrations,
            get_ui_preferences,
            update_ui_preferences,
            list_notification_failure_history,
            create_task,
            create_task_in_board_column,
            create_scheduled_task,
            create_task_list,
            update_task_list,
            delete_task_list,
            create_board_column,
            update_board_column,
            reorder_board_columns,
            delete_board_column,
            move_task_to_board_column,
            create_tag,
            update_tag,
            delete_tag,
            attach_tag_to_task,
            detach_tag_from_task,
            create_subtask,
            update_task,
            update_subtask,
            reorder_task_within_list,
            resize_scheduled_work_item,
            assign_work_schedule,
            move_scheduled_work_item,
            start_timer,
            start_standalone_pomodoro,
            pause_pomodoro,
            resume_pomodoro,
            complete_pomodoro_work_phase,
            start_pomodoro_break,
            skip_pomodoro_break,
            complete_pomodoro_break,
            cancel_pomodoro,
            pause_active_timer,
            resume_active_timer,
            stop_active_timer,
            complete_task,
            update_task_status,
            reopen_task,
            complete_subtask,
            reopen_subtask,
            toggle_task_favorite,
            archive_task,
            restore_archived_task,
            delete_task,
            delete_completed_tasks_in_board_column,
            delete_subtask,
            update_notification_display_mode,
            update_notifications_enabled,
            dispatch_due_notifications,
            create_sqlite_backup,
            restore_sqlite_backup,
            create_json_export,
            create_csv_export
        ])
        .run(tauri::generate_context!())
        .expect("TaskTimerの起動に失敗しました");
}
