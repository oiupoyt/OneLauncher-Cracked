use std::time::Duration;

use bytes::Bytes;
use freya::prelude::*;
use freya::query::{QueriesStorage, Query, QueryCapability, UseQuery, use_query};
use oneclient_core::LauncherError;
use serde::{Deserialize, Serialize};

use crate::AppAssets;

const SKIN_STALE: Duration = Duration::from_secs(60);
const SKIN_CLEAN: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CustomSkinQuery;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomSkinKeys {
    pub uuid: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SkinMetadata {
    pub is_slim: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CustomSkinData {
    pub bytes: Option<Bytes>,
    pub is_slim: bool,
    pub has_custom: bool,
}

impl QueryCapability for CustomSkinQuery {
    type Ok = CustomSkinData;
    type Err = LauncherError;
    type Keys = CustomSkinKeys;

    async fn run(&self, keys: &Self::Keys) -> Result<Self::Ok, Self::Err> {
        let Ok(path) = oneclient_common::paths::skin_file_path(&keys.uuid) else {
            return Ok(CustomSkinData::default());
        };

        if let Ok(bytes) = polyio::read(&path).await {
            if !bytes.is_empty() {
                let mut is_slim = false;
                if let Ok(meta_path) = oneclient_common::paths::skin_meta_path(&keys.uuid) {
                    if let Ok(meta_bytes) = polyio::read(&meta_path).await {
                        if let Ok(meta) = serde_json::from_slice::<SkinMetadata>(&meta_bytes) {
                            is_slim = meta.is_slim;
                        }
                    }
                }

                return Ok(CustomSkinData {
                    bytes: Some(Bytes::from(bytes)),
                    is_slim,
                    has_custom: true,
                });
            }
        }

        Ok(CustomSkinData::default())
    }
}

pub fn use_custom_skin(uuid: String) -> UseQuery<CustomSkinQuery> {
    use_query(
        Query::new(CustomSkinKeys { uuid }, CustomSkinQuery)
            .stale_time(SKIN_STALE)
            .clean_time(SKIN_CLEAN),
    )
}

pub async fn invalidate_skin_queries(uuid: Option<&str>) {
    if let Some(uuid) = uuid {
        QueriesStorage::<CustomSkinQuery>::invalidate_matching(CustomSkinKeys {
            uuid: uuid.to_string(),
        })
        .await;
    } else {
        QueriesStorage::<CustomSkinQuery>::invalidate_all().await;
    }
}

pub fn use_player_skin(uuid: String) -> (Bytes, bool) {
    let custom_query = use_custom_skin(uuid.clone());
    let custom_data = crate::hooks::settled_or_loading(&custom_query);

    let profile = super::use_player_profile(uuid.clone(), None::<String>);

    let (skin_url, is_slim) = crate::hooks::settled_or_loading(&profile)
        .map_or((None, false), |profile| (profile.skin_url, profile.is_slim));

    let skin_query = super::use_cached_image(skin_url.clone(), 256);

    let steve = use_memo(|| AppAssets::get_bytes("steve.png").unwrap_or_default());
    let alex = use_memo(|| AppAssets::get_bytes("alex.png").unwrap_or_default());

    let default_slim = (java_string_hash(&uuid) & 1) == 1;

    if let Some(custom) = custom_data {
        if custom.has_custom {
            if let Some(bytes) = custom.bytes {
                return (bytes, custom.is_slim);
            }
        }
    }

    match crate::hooks::loaded_image(skin_url.as_deref(), &skin_query) {
        Some((_, bytes)) => (bytes, is_slim),
        None if default_slim => (alex.read().clone(), true),
        None => (steve.read().clone(), false),
    }
}

pub fn java_string_hash(s: &str) -> i32 {
    let mut h: i32 = 0;
    for c in s.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(c as i32);
    }
    h
}

pub async fn save_account_skin(
    uuid: &str,
    username: Option<&str>,
    skin_bytes: &[u8],
    is_slim: bool,
) -> Result<(), String> {
    let img = image::load_from_memory(skin_bytes)
        .map_err(|e| format!("Invalid skin image format: {e}"))?;
    let (w, h) = (img.width(), img.height());

    let processed_bytes = if (w == 64 && h == 64) || (w == 128 && h == 128) {
        skin_bytes.to_vec()
    } else if w == 64 && h == 32 {
        let rgba = img.to_rgba8();
        let mut modern = image::RgbaImage::new(64, 64);
        image::imageops::overlay(&mut modern, &rgba, 0, 0);
        let mut out = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut out);
        modern
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        out
    } else {
        return Err(format!(
            "Invalid dimensions ({w}x{h}). Minecraft skins must be 64x64 or 64x32 PNG."
        ));
    };

    let skins_dir = oneclient_common::paths::skins_dir().map_err(|e| e.to_string())?;
    polyio::create_dir_all(&skins_dir)
        .await
        .map_err(|e| e.to_string())?;

    let skin_path = oneclient_common::paths::skin_file_path(uuid).map_err(|e| e.to_string())?;
    polyio::write(&skin_path, &processed_bytes)
        .await
        .map_err(|e| e.to_string())?;

    let meta_path = oneclient_common::paths::skin_meta_path(uuid).map_err(|e| e.to_string())?;
    let meta = SkinMetadata { is_slim };
    let meta_json = serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?;
    polyio::write(&meta_path, &meta_json)
        .await
        .map_err(|e| e.to_string())?;

    invalidate_skin_queries(Some(uuid)).await;

    // Sync to CustomSkinLoader directory if username is provided
    if let Some(uname) = username {
        if let Ok(dot_mc) = oneclient_common::paths::shared_minecraft_dir() {
            let csl_dir = dot_mc.join("CustomSkinLoader").join("skins");
            if polyio::create_dir_all(&csl_dir).await.is_ok() {
                let _ = polyio::write(csl_dir.join(format!("{uname}.png")), &processed_bytes).await;
            }
        }
    }

    Ok(())
}

pub async fn delete_account_skin(uuid: &str, username: Option<&str>) -> Result<(), String> {
    if let Ok(skin_path) = oneclient_common::paths::skin_file_path(uuid) {
        let _ = polyio::remove_file(skin_path).await;
    }
    if let Ok(meta_path) = oneclient_common::paths::skin_meta_path(uuid) {
        let _ = polyio::remove_file(meta_path).await;
    }
    if let Some(uname) = username {
        if let Ok(dot_mc) = oneclient_common::paths::shared_minecraft_dir() {
            let csl_skin = dot_mc
                .join("CustomSkinLoader")
                .join("skins")
                .join(format!("{uname}.png"));
            let _ = polyio::remove_file(csl_skin).await;
        }
    }
    invalidate_skin_queries(Some(uuid)).await;
    Ok(())
}

pub async fn fetch_skin_online(query: &str) -> Result<(Vec<u8>, bool), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("Please enter a username or skin URL".to_string());
    }

    let url = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://minotar.net/skin/{trimmed}")
    };

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download skin: {e}"))?;

    if !resp.status().is_success() {
        if !trimmed.starts_with("http") {
            let mineskin_url = format!("https://mineskin.eu/skin/{trimmed}");
            if let Ok(m_resp) = client.get(&mineskin_url).send().await {
                if m_resp.status().is_success() {
                    if let Ok(bytes) = m_resp.bytes().await {
                        let is_slim = (java_string_hash(trimmed) & 1) == 1;
                        return Ok((bytes.to_vec(), is_slim));
                    }
                }
            }
        }
        return Err(format!("Could not find skin for \"{trimmed}\""));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let _ = image::load_from_memory(&bytes)
        .map_err(|e| format!("Downloaded file is not a valid image: {e}"))?;
    let is_slim = (java_string_hash(trimmed) & 1) == 1;
    Ok((bytes.to_vec(), is_slim))
}
