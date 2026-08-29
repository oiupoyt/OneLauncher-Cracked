use bytes::Bytes;
use freya::query::{QueriesStorage, Query, QueryCapability, UseQuery, use_query};
use oneclient_common::paths::{shared_minecraft_dir, skin_file_path, skin_meta_path, skins_dir};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::AppAssets;

const SKIN_STALE: Duration = Duration::from_secs(5 * 60);
const SKIN_CLEAN: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomSkinKeys {
    pub uuid: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomSkinData {
    pub uuid: String,
    pub has_custom: bool,
    pub bytes: Option<Bytes>,
    pub is_slim: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SkinMetadata {
    pub is_slim: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CustomSkinQuery;

impl QueryCapability for CustomSkinQuery {
    type Ok = CustomSkinData;
    type Err = String;
    type Keys = CustomSkinKeys;

    async fn run(&self, keys: &Self::Keys) -> Result<Self::Ok, Self::Err> {
        let uuid = &keys.uuid;
        if uuid.is_empty() {
            return Ok(CustomSkinData {
                uuid: String::new(),
                has_custom: false,
                bytes: None,
                is_slim: false,
            });
        }

        let skin_path = skin_file_path(uuid).map_err(|e| e.to_string())?;
        let meta_path = skin_meta_path(uuid).map_err(|e| e.to_string())?;

        if !skin_path.exists() {
            return Ok(CustomSkinData {
                uuid: uuid.clone(),
                has_custom: false,
                bytes: None,
                is_slim: false,
            });
        }

        let bytes = polyio::read(&skin_path)
            .await
            .map_err(|e| format!("Failed to read custom skin: {e}"))?;

        let is_slim = if meta_path.exists() {
            polyio::read_to_string(&meta_path)
                .await
                .ok()
                .and_then(|s| serde_json::from_str::<SkinMetadata>(&s).ok())
                .map(|m| m.is_slim)
                .unwrap_or(false)
        } else {
            false
        };

        Ok(CustomSkinData {
            uuid: uuid.clone(),
            has_custom: true,
            bytes: Some(Bytes::from(bytes)),
            is_slim,
        })
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

    let steve = AppAssets::get_bytes("steve.png").unwrap_or_default();
    let alex = AppAssets::get_bytes("alex.png").unwrap_or_default();

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
        None if default_slim => (alex, true),
        None => (steve, false),
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
    let skins_folder = skins_dir().map_err(|e| e.to_string())?;
    polyio::create_dir_all(&skins_folder)
        .await
        .map_err(|e| e.to_string())?;

    let skin_path = skin_file_path(uuid).map_err(|e| e.to_string())?;
    let meta_path = skin_meta_path(uuid).map_err(|e| e.to_string())?;

    let img = image::load_from_memory(skin_bytes)
        .map_err(|e| format!("Invalid image format: {e}"))?
        .to_rgba8();

    let (w, h) = (img.width(), img.height());

    let normalized_img = if w == 64 && h == 32 {
        let mut new_img = image::RgbaImage::new(64, 64);
        image::imageops::overlay(&mut new_img, &img, 0, 0);
        new_img
    } else if w == 64 && h == 64 {
        img
    } else {
        return Err(format!(
            "Invalid Minecraft skin dimensions: {w}x{h}. Must be 64x64 or 64x32."
        ));
    };

    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    normalized_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {e}"))?;

    polyio::write(&skin_path, &png_bytes)
        .await
        .map_err(|e| format!("Failed to write skin file: {e}"))?;

    let meta = SkinMetadata { is_slim };
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize metadata: {e}"))?;

    polyio::write(&meta_path, meta_json.as_bytes())
        .await
        .map_err(|e| format!("Failed to write skin metadata: {e}"))?;

    if let Some(uname) = username {
        if !uname.is_empty() {
            let _ = sync_custom_skin_loader(uname, &png_bytes).await;
        }
    }

    invalidate_skin_queries(Some(uuid)).await;

    Ok(())
}

pub async fn delete_account_skin(uuid: &str, username: Option<&str>) -> Result<(), String> {
    let skin_path = skin_file_path(uuid).map_err(|e| e.to_string())?;
    let meta_path = skin_meta_path(uuid).map_err(|e| e.to_string())?;

    if skin_path.exists() {
        let _ = polyio::remove_file(&skin_path).await;
    }
    if meta_path.exists() {
        let _ = polyio::remove_file(&meta_path).await;
    }

    if let Some(uname) = username {
        if !uname.is_empty() {
            let _ = remove_custom_skin_loader(uname).await;
        }
    }

    invalidate_skin_queries(Some(uuid)).await;

    Ok(())
}

async fn sync_custom_skin_loader(username: &str, skin_png: &[u8]) -> Result<(), String> {
    if let Ok(mc_dir) = shared_minecraft_dir() {
        let csl_skins_dir = mc_dir.join("CustomSkinLoader").join("skins");
        let _ = polyio::create_dir_all(&csl_skins_dir).await;
        let target = csl_skins_dir.join(format!("{username}.png"));
        let _ = polyio::write(&target, skin_png).await;
    }
    Ok(())
}

async fn remove_custom_skin_loader(username: &str) -> Result<(), String> {
    if let Ok(mc_dir) = shared_minecraft_dir() {
        let target = mc_dir
            .join("CustomSkinLoader")
            .join("skins")
            .join(format!("{username}.png"));
        if target.exists() {
            let _ = polyio::remove_file(&target).await;
        }
    }
    Ok(())
}
