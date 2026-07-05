use crate::domain::{
    notification::NotificationDisplayMode,
    task::WorkStatus,
    timer::{WorkTargetRef, WorkTargetType},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarMarker {
    PlannedStart,
    Due,
    ActiveTimer,
}

impl CalendarMarker {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlannedStart => "planned_start",
            Self::Due => "due",
            Self::ActiveTimer => "active_timer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekCalendarItem {
    pub id: String,
    pub target: WorkTargetRef,
    pub title: String,
    pub date: String,
    pub marker: CalendarMarker,
    pub status: WorkStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTimer {
    pub id: String,
    pub target: WorkTargetRef,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub elapsed_seconds: Option<i64>,
    pub deleted_at: Option<String>,
    pub created_at: String,
}

pub type RepositoryResult<T> = Result<T, String>;

pub trait CalendarRepository {
    fn list_week_calendar_items(
        &self,
        week_start_date: &str,
    ) -> RepositoryResult<Vec<WeekCalendarItem>>;
}

pub trait TimerRepository {
    fn get_active_timer(&self) -> RepositoryResult<Option<ActiveTimer>>;
}

pub trait NotificationPreferenceRepository {
    fn get_notification_display_mode(&self) -> RepositoryResult<NotificationDisplayMode>;
}

pub fn target_ref(target_type: WorkTargetType, id: String) -> WorkTargetRef {
    WorkTargetRef { target_type, id }
}
