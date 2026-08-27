use serde::{Deserialize, Serialize};
use uuid::{Builder, Uuid, Variant, Version};
use md5::{Md5, Digest};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub username: String,
    pub uuid: Uuid,
    pub access_token: String,
    pub user_type: String,
}

impl OfflineAccount {
    pub fn new(username: &str) -> Result<Self, OfflineAuthError> {
        let trimmed = username.trim();
        if trimmed.is_empty() { return Err(OfflineAuthError::EmptyUsername); }
        if trimmed.len() > 16 { return Err(OfflineAuthError::UsernameTooLong); }
        if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(OfflineAuthError::InvalidCharacters);
        }

        let uuid = generate_offline_uuid(trimmed);
        Ok(Self {
            username: trimmed.to_string(),
            uuid,
            access_token: "offline".to_string(),
            user_type: "offline".to_string(),
        })
    }
}

pub fn generate_offline_uuid(username: &str) -> Uuid {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{}", username).as_bytes());
    let result = hasher.finalize();

    let mut builder = Builder::from_slice(&result).expect("MD5 is 16 bytes");
    builder.set_version(Version::Md5);
    builder.set_variant(Variant::RFC4122);
    builder.into_uuid()
}

#[derive(Debug, Error)]
pub enum OfflineAuthError {
    #[error("Username cannot be empty")]
    EmptyUsername,
    #[error("Username cannot exceed 16 characters")]
    UsernameTooLong,
    #[error("Invalid characters in username")]
    InvalidCharacters,
}
