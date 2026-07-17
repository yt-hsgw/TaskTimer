#![allow(dead_code)]

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::application::notification::{
    LocalNotificationGateway, LocalNotificationMessage, NativeNotificationRegistrationGateway,
    NativeNotificationRegistrationRequest,
};

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

pub struct TauriNativeNotificationRegistrationGateway {
    app_id: String,
}

impl TauriNativeNotificationRegistrationGateway {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_id: app_handle.config().identifier.clone(),
        }
    }
}

impl NativeNotificationRegistrationGateway for TauriNativeNotificationRegistrationGateway {
    fn is_available(&self) -> bool {
        cfg!(windows)
    }

    fn register_or_replace(
        &self,
        request: &NativeNotificationRegistrationRequest,
    ) -> Result<String, String> {
        #[cfg(windows)]
        {
            windows_scheduled_toast::register_or_replace(&self.app_id, request)
        }
        #[cfg(not(windows))]
        {
            let _ = request;
            Err("Windowsネイティブ通知登録はこのOSでは利用できません".to_string())
        }
    }

    fn cancel(&self, os_registration_id: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            windows_scheduled_toast::cancel(&self.app_id, os_registration_id)
        }
        #[cfg(not(windows))]
        {
            let _ = os_registration_id;
            Err("Windowsネイティブ通知登録はこのOSでは利用できません".to_string())
        }
    }
}

fn deterministic_os_registration_id(registration_id: &str) -> String {
    format!("tasktimer:{registration_id}")
}

fn windows_datetime_ticks_from_iso8601(value: &str) -> Result<i64, String> {
    let datetime = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|error| format!("通知予定時刻の形式が不正です: {error}"))?;
    let unix_ticks = datetime.unix_timestamp_nanos() / 100;
    let windows_epoch_offset_ticks = 116_444_736_000_000_000_i128;
    (unix_ticks + windows_epoch_offset_ticks)
        .try_into()
        .map_err(|_| "通知予定時刻をWindows DateTimeへ変換できません".to_string())
}

fn escape_toast_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn toast_xml(title: &str, body: &str) -> String {
    let title = escape_toast_xml(title);
    let body = escape_toast_xml(body);
    if body.is_empty() {
        format!(
            r#"<toast><visual><binding template="ToastGeneric"><text>{title}</text></binding></visual></toast>"#
        )
    } else {
        format!(
            r#"<toast><visual><binding template="ToastGeneric"><text>{title}</text><text>{body}</text></binding></visual></toast>"#
        )
    }
}

#[cfg(windows)]
mod windows_scheduled_toast {
    use super::{deterministic_os_registration_id, toast_xml, windows_datetime_ticks_from_iso8601};
    use crate::application::notification::NativeNotificationRegistrationRequest;
    use windows::{
        core::HSTRING,
        Data::Xml::Dom::XmlDocument,
        Foundation::DateTime,
        UI::Notifications::{ScheduledToastNotification, ToastNotificationManager},
    };

    pub fn register_or_replace(
        app_id: &str,
        request: &NativeNotificationRegistrationRequest,
    ) -> Result<String, String> {
        if let Some(existing_os_registration_id) = request.existing_os_registration_id.as_deref() {
            cancel(app_id, existing_os_registration_id)?;
        }

        let os_registration_id = deterministic_os_registration_id(&request.registration_id);
        let document = XmlDocument::new()
            .map_err(|error| format!("Windows通知XMLを作成できません: {error}"))?;
        document
            .LoadXml(&HSTRING::from(toast_xml(&request.title, &request.body)))
            .map_err(|error| format!("Windows通知XMLを読み込めません: {error}"))?;

        let delivery_time = DateTime {
            UniversalTime: windows_datetime_ticks_from_iso8601(&request.notify_at)?,
        };
        let scheduled =
            ScheduledToastNotification::CreateScheduledToastNotification(&document, delivery_time)
                .map_err(|error| format!("Windows通知予約を作成できません: {error}"))?;
        let os_id = HSTRING::from(os_registration_id.as_str());
        scheduled
            .SetId(&os_id)
            .map_err(|error| format!("Windows通知予約IDを設定できません: {error}"))?;
        scheduled
            .SetTag(&os_id)
            .map_err(|error| format!("Windows通知予約タグを設定できません: {error}"))?;
        scheduled
            .SetGroup(&HSTRING::from("tasktimer"))
            .map_err(|error| format!("Windows通知予約グループを設定できません: {error}"))?;

        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
            .map_err(|error| format!("Windows通知Notifierを作成できません: {error}"))?;
        notifier
            .AddToSchedule(&scheduled)
            .map_err(|error| format!("Windows通知予約を登録できません: {error}"))?;

        Ok(os_registration_id)
    }

    pub fn cancel(app_id: &str, os_registration_id: &str) -> Result<(), String> {
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
            .map_err(|error| format!("Windows通知Notifierを作成できません: {error}"))?;
        let scheduled = notifier
            .GetScheduledToastNotifications()
            .map_err(|error| format!("Windows通知予約一覧を取得できません: {error}"))?;
        let scheduled_count = scheduled
            .Size()
            .map_err(|error| format!("Windows通知予約件数を取得できません: {error}"))?;

        for index in 0..scheduled_count {
            let notification = scheduled
                .GetAt(index)
                .map_err(|error| format!("Windows通知予約を取得できません: {error}"))?;
            let id = notification
                .Id()
                .map_err(|error| format!("Windows通知予約IDを取得できません: {error}"))?;
            if id.to_string() == os_registration_id {
                notifier
                    .RemoveFromSchedule(&notification)
                    .map_err(|error| format!("Windows通知予約を解除できません: {error}"))?;
                return Ok(());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_datetime_ticks_from_iso8601_converts_unix_epoch() {
        assert_eq!(
            windows_datetime_ticks_from_iso8601("1970-01-01T00:00:00Z").expect("epoch"),
            116_444_736_000_000_000
        );
    }

    #[test]
    fn toast_xml_escapes_user_text() {
        let xml = toast_xml("a<b&c", "\"memo\"");
        assert!(xml.contains("a&lt;b&amp;c"));
        assert!(xml.contains("&quot;memo&quot;"));
    }

    #[test]
    fn deterministic_os_registration_id_is_stable() {
        assert_eq!(
            deterministic_os_registration_id("registration-1"),
            "tasktimer:registration-1"
        );
    }
}
