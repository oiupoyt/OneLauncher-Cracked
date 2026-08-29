use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use base64::Engine;
use oneclient_common::paths::{authlib_injector_path, shared_minecraft_dir, skin_file_path, skin_meta_path, skins_dir};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const AUTHLIB_INJECTOR_URL: &str =
    "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.5/authlib-injector-1.2.5.jar";

static SKIN_SERVER_PORT: OnceLock<u16> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SkinMetadata {
    pub is_slim: bool,
}

/// Prepares the authlib-injector jar, downloading it if not present.
pub async fn prepare_authlib_injector() -> Result<PathBuf, String> {
    let jar_path = authlib_injector_path().map_err(|e| e.to_string())?;
    if jar_path.exists() && jar_path.metadata().map(|m| m.len() > 10000).unwrap_or(false) {
        return Ok(jar_path);
    }

    if let Some(parent) = jar_path.parent() {
        polyio::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

    let client = reqwest::Client::builder()
        .user_agent("OneLauncher-Cracked/2.2.3")
        .build()
        .map_err(|e| e.to_string())?;

    tracing::info!("downloading authlib-injector for offline skin support");
    let resp = client
        .get(AUTHLIB_INJECTOR_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to download authlib-injector: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Failed to download authlib-injector: HTTP status {}",
            resp.status()
        ));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read authlib-injector bytes: {e}"))?;

    polyio::write(&jar_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write authlib-injector: {e}"))?;

    tracing::info!(path = %jar_path.display(), "authlib-injector ready");
    Ok(jar_path)
}

/// Ensures the local skin server is running in the background and returns its bound port.
pub async fn ensure_skin_server() -> Result<u16, String> {
    if let Some(&port) = SKIN_SERVER_PORT.get() {
        return Ok(port);
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind local skin server listener: {e}"))?;

    let port = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .port();

    let _ = SKIN_SERVER_PORT.set(port);

    tokio::spawn(async move {
        tracing::info!(port, "local skin server listening");
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        let _ = handle_connection(stream, port).await;
                    });
                }
                Err(err) => {
                    tracing::warn!(error = %err, "skin server accept error");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    });

    Ok(port)
}

async fn handle_connection(mut stream: TcpStream, port: u16) -> Result<(), std::io::Error> {
    let mut buffer = [0u8; 4096];
    let n = stream.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buffer[..n]);
    let mut lines = request_str.lines();
    let first_line = match lines.next() {
        Some(l) => l,
        None => return Ok(()),
    };

    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let uri = parts.next().unwrap_or("");

    if method != "GET" && method != "HEAD" {
        let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    let path = uri.split('?').next().unwrap_or(uri);

    if path == "/" || path == "/api/yggdrasil" || path == "/api/yggdrasil/" {
        let body = format!(
            r#"{{"meta":{{"serverName":"OneLauncher-Cracked","implementationName":"onelauncher-skin-server","implementationVersion":"2.2.3"}},"skinDomains":["127.0.0.1","localhost"]}}"#
        );
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(header.as_bytes()).await?;
        return Ok(());
    }

    if let Some(uuid_raw) = path.strip_prefix("/session/minecraft/profile/") {
        let uuid_str = uuid_raw.trim_matches('/');
        if let Some(json_body) = build_profile_response(uuid_str, port).await {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(header.as_bytes()).await?;
        } else {
            let response =
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).await?;
        }
        return Ok(());
    }

    if let Some(texture_raw) = path.strip_prefix("/textures/") {
        let texture_id = texture_raw.trim_matches('/').trim_end_matches(".png");
        if let Some(png_bytes) = load_skin_png(texture_id).await {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                png_bytes.len()
            );
            stream.write_all(header.as_bytes()).await?;
            if method == "GET" {
                stream.write_all(&png_bytes).await?;
            }
        } else {
            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).await?;
        }
        return Ok(());
    }

    let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

fn to_dashed_uuid(s: &str) -> String {
    let clean = s.replace('-', "");
    if clean.len() == 32 {
        format!(
            "{}-{}-{}-{}-{}",
            &clean[0..8],
            &clean[8..12],
            &clean[12..16],
            &clean[16..20],
            &clean[20..32]
        )
    } else {
        s.to_string()
    }
}

fn to_undashed_uuid(s: &str) -> String {
    s.replace('-', "")
}

async fn find_skin_path(uuid_or_name: &str) -> Option<PathBuf> {
    let dashed = to_dashed_uuid(uuid_or_name);
    let undashed = to_undashed_uuid(uuid_or_name);

    if let Ok(p) = skin_file_path(&dashed) {
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(p) = skin_file_path(&undashed) {
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(p) = skin_file_path(uuid_or_name) {
        if p.exists() {
            return Some(p);
        }
    }

    // Fallback: check if username matched
    if let Ok(dir) = skins_dir() {
        let name_path = dir.join(format!("{uuid_or_name}.png"));
        if name_path.exists() {
            return Some(name_path);
        }
    }

    None
}

async fn load_skin_png(uuid_or_name: &str) -> Option<Vec<u8>> {
    let path = find_skin_path(uuid_or_name).await?;
    polyio::read(&path).await.ok()
}

async fn build_profile_response(uuid_input: &str, port: u16) -> Option<String> {
    let dashed = to_dashed_uuid(uuid_input);
    let undashed = to_undashed_uuid(uuid_input);

    let _skin_path = find_skin_path(uuid_input).await?;

    let is_slim = if let Ok(meta_p) = skin_meta_path(&dashed) {
        if meta_p.exists() {
            polyio::read_to_string(&meta_p)
                .await
                .ok()
                .and_then(|s| serde_json::from_str::<SkinMetadata>(&s).ok())
                .map(|m| m.is_slim)
                .unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

    let skin_url = format!("http://127.0.0.1:{port}/textures/{undashed}.png");

    let model_json = if is_slim {
        r#","metadata":{"model":"slim"}"#
    } else {
        ""
    };

    let texture_payload = format!(
        r#"{{"timestamp":1700000000000,"profileId":"{undashed}","profileName":"Player","textures":{{"SKIN":{{"url":"{skin_url}"{model_json}}}}}}}"#
    );

    let encoded_textures =
        base64::engine::general_purpose::STANDARD.encode(texture_payload.as_bytes());

    let profile_json = format!(
        r#"{{"id":"{undashed}","name":"Player","properties":[{{"name":"textures","value":"{encoded_textures}"}}]}}"#
    );

    Some(profile_json)
}

/// Synchronizes the custom skin to various mod directory layouts (CustomSkinLoader, OfflineSkins, etc.)
pub async fn sync_offline_skins(
    game_dir: &Path,
    account_uuid: &str,
    account_username: &str,
) {
    if let Some(png_bytes) = load_skin_png(account_uuid).await {
        let targets = [
            game_dir.join("CustomSkinLoader").join("skins").join(format!("{account_username}.png")),
            game_dir.join("CustomSkinLoader").join("skins").join(format!("{account_uuid}.png")),
            game_dir.join("config").join("offlineskins").join(format!("{account_username}.png")),
            game_dir.join("config").join("offlineskins").join(format!("{account_uuid}.png")),
            game_dir.join("cachedImages").join("skins").join(format!("{account_uuid}.png")),
        ];

        for target in targets {
            if let Some(parent) = target.parent() {
                let _ = polyio::create_dir_all(parent).await;
            }
            let _ = polyio::write(&target, &png_bytes).await;
        }

        if let Ok(shared) = shared_minecraft_dir() {
            let shared_targets = [
                shared.join("CustomSkinLoader").join("skins").join(format!("{account_username}.png")),
                shared.join("config").join("offlineskins").join(format!("{account_username}.png")),
            ];
            for target in shared_targets {
                if let Some(parent) = target.parent() {
                    let _ = polyio::create_dir_all(parent).await;
                }
                let _ = polyio::write(&target, &png_bytes).await;
            }
        }
    }
}
