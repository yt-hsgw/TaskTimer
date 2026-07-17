#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNotificationMessage {
    pub title: String,
    pub body: String,
}

pub trait LocalNotificationGateway {
    fn send(&self, message: &LocalNotificationMessage) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNotificationRegistrationRequest {
    pub registration_id: String,
    pub existing_os_registration_id: Option<String>,
    pub title: String,
    pub body: String,
    pub notify_at: String,
}

pub trait NativeNotificationRegistrationGateway {
    fn is_available(&self) -> bool;

    fn register_or_replace(
        &self,
        request: &NativeNotificationRegistrationRequest,
    ) -> Result<String, String>;

    fn cancel(&self, os_registration_id: &str) -> Result<(), String>;
}
