pub mod microsoft;
pub mod offline;

pub use offline::{OfflineAccount, generate_offline_uuid};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AccountType {
    Microsoft { refresh_token_key: String },
    Offline,
}
