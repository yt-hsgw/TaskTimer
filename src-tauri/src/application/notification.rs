#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNotificationMessage {
    pub title: String,
    pub body: String,
}

pub trait LocalNotificationGateway {
    fn send(&self, message: &LocalNotificationMessage) -> Result<(), String>;
}
