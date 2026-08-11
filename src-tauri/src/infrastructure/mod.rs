//! SQLite、OS通知、システム時刻を実装するInfrastructure adapter群。
//! 外部ネットワーク通信は追加しない。

pub mod clock;
pub mod notification;
pub mod sqlite;
