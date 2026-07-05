#![allow(dead_code)]

pub trait LocalNotificationGateway {
    fn register(&self, title: &str, body: &str, notify_at: &str) -> Result<(), String>;
}

pub struct NotImplementedNotificationGateway;

impl LocalNotificationGateway for NotImplementedNotificationGateway {
    fn register(&self, _title: &str, _body: &str, _notify_at: &str) -> Result<(), String> {
        Err("通知ゲートウェイはまだ実装されていません".to_string())
    }
}
