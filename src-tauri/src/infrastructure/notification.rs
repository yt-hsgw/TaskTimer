#![allow(dead_code)]

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::application::notification::{LocalNotificationGateway, LocalNotificationMessage};

pub struct TauriLocalNotificationGateway {
    app_handle: AppHandle,
}

impl TauriLocalNotificationGateway {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl LocalNotificationGateway for TauriLocalNotificationGateway {
    fn send(&self, message: &LocalNotificationMessage) -> Result<(), String> {
        let mut builder = self
            .app_handle
            .notification()
            .builder()
            .title(&message.title);
        if !message.body.is_empty() {
            builder = builder.body(&message.body);
        }

        builder
            .show()
            .map(|_| ())
            .map_err(|error| format!("OS通知を送信できません: {error}"))
    }
}
