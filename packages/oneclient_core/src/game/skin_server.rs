use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use base64::Engine;
use oneclient_common::paths::{
    authlib_injector_path, shared_minecraft_dir, skin_file_path, skin_meta_path, skins_dir,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use rsa::pkcs8::DecodePrivateKey;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};

type MojangCacheEntry = (Option<String>, Instant);
type MojangCacheMap = HashMap<String, MojangCacheEntry>;

static AUTHLIB_INJECTOR_BYTES: &[u8] = include_bytes!("../../assets/authlib-injector.jar");
static SKIN_SERVER_PORT: OnceLock<u16> = OnceLock::new();
static REGISTERED_ACCOUNTS: OnceLock<RwLock<HashMap<String, AccountRegistration>>> = OnceLock::new();
static MOJANG_CACHE: OnceLock<RwLock<MojangCacheMap>> = OnceLock::new();

pub static SKIN_SERVER_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2ZEWcf6IB84Atm2pnE/N\na19ipdSUpXjMYWaloNhP5ygsMB63i1T55RxaMeVoX/CLY1cFI14+BqgeHGYHOLyo\nVoLz1ylC1QvI99Q1fyl0caTD41K11T5l+EGs5gpepGbQ11tRhtTFNpZg016W83t9\nQMMqpVzSETfK65bu5jOeipvf9TAlXNCpRPZBXe12+kJ5H1t3H9NT2KMUpcYPER7p\nIeIMOgiUwCCBqlXVhoKqjTvplKmyaY/qP3zBdLDQ/+mA1i9SOM8529vi452g+ftP\nkUK+AP52KBb4MnmrwasnH9JdzmS8t2Kl17HYORZ6RpHAPlF4DEC7QDEhvJRLvf1N\nbQIDAQAB\n-----END PUBLIC KEY-----";

static SKIN_SERVER_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDZkRZx/ogHzgC2\nbamcT81rX2Kl1JSleMxhZqWg2E/nKCwwHreLVPnlHFox5Whf8ItjVwUjXj4GqB4c\nZgc4vKhWgvPXKULVC8j31DV/KXRxpMPjUrXVPmX4QazmCl6kZtDXW1GG1MU2lmDT\nXpbze31AwyqlXNIRN8rrlu7mM56Km9/1MCVc0KlE9kFd7Xb6QnkfW3cf01PYoxSl\nxg8RHukh4gw6CJTAIIGqVdWGgqqNO+mUqbJpj+o/fMF0sND/6YDWL1I4zznb2+Lj\nnaD5+0+RQr4A/nYoFvgyeavBqycf0l3OZLy3YqXXsdg5FnpGkcA+UXgMQLtAMSG8\nlEu9/U1tAgMBAAECggEACCDgDk08ptH++9HYCuN+YLsVE+3/ybjcJe1wVbSPM6sw\nD3IuWFHJ7lHjWsbf4em6Q3FeW0ZrmdMRIO186pU90tGcq5a6jPweO4gdoY0acR5/\nRRBzg66Ln71QaN3NUGYY+lrKjneHkLUIlA0OJbWg5dkE0F3J6WPEvI2MimQ2UaZU\nkqrGKT7QiAhJSlBSAi4dcDXQbSyFRbCm1hw3goL+hpOfFUVymR0TLFqAl8p0uaei\nnGOt4A3g84xx7zHQx4iUVzanX7mGnKDfEHPB1wohvKkn37vGIocAT43kt/AxJS8p\nzMgy1UyyzIwc6vjuDOqLKqTl6dqNvftzucBHTp8NHwKBgQDtUlYCNsyptDwLXHC7\nmIQd4OJIihZxoOwbPx2gzahpaymyeu3/2w6hA2BlqS1/PFpkhe1igOsYkqUWMyNs\nwA9ob4df9CQhPe787q12YL6LahY73A/Qfgn5+6dW8l9pYPyBgwmUd/NMdkQjZILH\nHTxB8eX1F2IsxAKoQ0g8unQ2OwKBgQDqsLiysueH3gL/u19RksQy9PIVi6dnSuiH\nq93zIKVjKcQmAOrQboo9w8wXbwX3z8WGGbxWNbnIEFYTiKzQD2fviserV1d4Xz9E\nOwIDuc8PdBWbgpjSfIDbBqc6AmE34JLyn1AdX9fkhBjquAy370v4ZdHdG5wnpxSg\nw8qQCy7IdwKBgCA5lOo6DLJigeC9DaW7gP0Zo0BcV83YHxdYC6rhIiQmZAQTQywB\nz8u3TKihP0dOp6uMr/43KTUt/HK2QPIsZis1MbmqyhklcsUvl6hCXL1Li3dXW2Jh\nKvOh40ggIyqI++COLYfWfdf9GyV/KW7mHl+J/EK6iR8xAndco3tzigIvAoGAXBRE\nExCwWJVZld59EnND+T4zcRKe9p7kRr6+0TJA0XxEkiiP+IE2Se91Nsh/je/97pRQ\nWX6wynbmXrmkG+m/fLN1jZsyHW85UlrYen+/Zq/D/oSp0wO4RrcAi3j9jb/Vx82L\n0EqXWPgfEpBtpQkFRIsmYNsBVGlwZXcMFaHdlBcCgYAsBi2KNp3/88XdWxODFpEu\nZ5YjqmhGEAInCYTQOquYHFBjVTrV04I2Zr031Z6iETopm/9nLZ15VMjbZyoZWS3B\nAteqz/MfyDOUN3U4xrBr6nxD/HcpNoWeLXyzGnRya8K+/1vz0hWP/e5Q/5KvCGAo\npmSM49/FvJ7y20c1x1fq1A==\n-----END PRIVATE KEY-----";

static SIGNING_KEY: OnceLock<RsaPrivateKey> = OnceLock::new();

pub fn sign_sha1_with_rsa(data: &[u8]) -> Option<String> {
    use sha1::Digest;
    let key = SIGNING_KEY.get_or_init(|| {
        RsaPrivateKey::from_pkcs8_pem(SKIN_SERVER_PRIVATE_KEY_PEM)
            .expect("valid embedded private key")
    });
    let mut hasher = sha1::Sha1::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let signing_scheme = Pkcs1v15Sign::new::<sha1::Sha1>();
    let sig_bytes = key.sign(signing_scheme, &digest).ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(sig_bytes))
}

#[derive(Clone, Debug)]
pub struct AccountRegistration {
    pub uuid: String,
    pub undashed_uuid: String,
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SkinMetadata {
    pub is_slim: bool,
}

/// Registers an active launch account with its UUID and username so the local
/// skin server can immediately resolve textures and profile properties for it.
pub fn register_launch_account(uuid: &str, username: &str) {
    let accounts = REGISTERED_ACCOUNTS.get_or_init(|| RwLock::new(HashMap::new()));
    let dashed = to_dashed_uuid(uuid);
    let undashed = to_undashed_uuid(uuid);
    let reg = AccountRegistration {
        uuid: dashed.clone(),
        undashed_uuid: undashed.clone(),
        username: username.to_string(),
    };
    let mut map = accounts.write();
    map.insert(dashed, reg.clone());
    map.insert(undashed, reg.clone());
    map.insert(username.to_lowercase(), reg);
}

/// Ensures the authlib-injector jar is extracted to disk from embedded bytes.
/// Operates 100% offline with zero network dependencies.
pub async fn prepare_authlib_injector() -> Result<PathBuf, String> {
    let jar_path = authlib_injector_path().map_err(|e| e.to_string())?;

    let is_valid = if let Ok(meta) = tokio::fs::metadata(&jar_path).await {
        meta.len() > 100_000
    } else {
        false
    };

    if !is_valid {
        if let Some(parent) = jar_path.parent() {
            polyio::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        polyio::write(&jar_path, AUTHLIB_INJECTOR_BYTES)
            .await
            .map_err(|e| e.to_string())?;
        tracing::info!(path = %jar_path.display(), "extracted embedded authlib-injector jar");
    }

    Ok(jar_path)
}

/// Starts the local skin server on a random local port if not already started.
pub async fn ensure_skin_server() -> Result<u16, String> {
    if let Some(&port) = SKIN_SERVER_PORT.get() {
        return Ok(port);
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("failed to bind local skin server: {e}"))?;

    let port = listener
        .local_addr()
        .map_err(|e| format!("failed to get listener port: {e}"))?
        .port();

    let _ = SKIN_SERVER_PORT.set(port);

    tokio::spawn(async move {
        tracing::info!(port, "local offline skin server listening");
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        let _ = handle_connection(stream, port).await;
                    });
                }
                Err(err) => {
                    tracing::warn!(error = %err, "skin server accept error");
                    tokio::time::sleep(Duration::from_millis(50)).await;
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

    if method != "GET" && method != "HEAD" && method != "POST" {
        let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    let path = uri.split('?').next().unwrap_or(uri);

    // 1. Root / Yggdrasil API metadata
    if path == "/" || path == "/api/yggdrasil" || path == "/api/yggdrasil/" {
        let body = serde_json::json!({
            "meta": {
                "serverName": "OneLauncher-Cracked",
                "implementationName": "onelauncher-skin-server",
                "implementationVersion": env!("CARGO_PKG_VERSION")
            },
            "skinDomains": ["127.0.0.1", "localhost"],
            "signaturePublickey": SKIN_SERVER_PUBLIC_KEY
        }).to_string();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(header.as_bytes()).await?;
        return Ok(());
    }

    // 2. Public keys endpoint (for Minecraft 1.19+ profile signature verification)
    if path == "/publickeys" || path == "/minecraftservices/publickeys" {
        let key_b64 = "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2ZEWcf6IB84Atm2pnE/Na19ipdSUpXjMYWaloNhP5ygsMB63i1T55RxaMeVoX/CLY1cFI14+BqgeHGYHOLyoVoLz1ylC1QvI99Q1fyl0caTD41K11T5l+EGs5gpepGbQ11tRhtTFNpZg016W83t9QMMqpVzSETfK65bu5jOeipvf9TAlXNCpRPZBXe12+kJ5H1t3H9NT2KMUpcYPER7pIeIMOgiUwCCBqlXVhoKqjTvplKmyaY/qP3zBdLDQ/+mA1i9SOM8529vi452g+ftPkUK+AP52KBb4MnmrwasnH9JdzmS8t2Kl17HYORZ6RpHAPlF4DEC7QDEhvJRLvf1NbQIDAQAB";
        let body = serde_json::json!({
            "profilePropertyKeys": [{"publicKey": key_b64}],
            "playerCertificateKeys": [{"publicKey": key_b64}]
        }).to_string();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(header.as_bytes()).await?;
        return Ok(());
    }

    // 3. Profiles by names endpoint (bulk profile lookup)
    if path == "/api/profiles/minecraft" || path == "/api/profiles/minecraft/" {
        let mut profiles = Vec::new();
        if let Some(accounts) = REGISTERED_ACCOUNTS.get() {
            let read = accounts.read();
            for reg in read.values() {
                if !profiles.iter().any(|p: &serde_json::Value| p["id"] == reg.undashed_uuid) {
                    profiles.push(serde_json::json!({
                        "id": reg.undashed_uuid,
                        "name": reg.username,
                    }));
                }
            }
        }
        let body = serde_json::to_string(&profiles).unwrap_or_else(|_| "[]".to_string());
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(header.as_bytes()).await?;
        return Ok(());
    }

    // 4. Profile request: /sessionserver/session/minecraft/profile/<uuid> or /session/minecraft/profile/<uuid>
    let profile_uuid = path
        .strip_prefix("/sessionserver/session/minecraft/profile/")
        .or_else(|| path.strip_prefix("/session/minecraft/profile/"));

    if let Some(uuid_raw) = profile_uuid {
        let uuid_str = uuid_raw.trim_matches('/');

        // Check if this is one of our local accounts with a custom skin
        if let Some(json_body) = build_profile_response(uuid_str, port).await {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(header.as_bytes()).await?;
            return Ok(());
        }

        // For other players on the server, check Mojang session server so official players keep their skins
        if let Some(mojang_body) = fetch_mojang_profile(uuid_str).await {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                mojang_body.len(),
                mojang_body
            );
            stream.write_all(header.as_bytes()).await?;
            return Ok(());
        }

        // Offline / unknown player: return 204 No Content -> vanilla Minecraft renders default Steve/Alex
        let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    // 5. Texture request: /textures/<id>.png
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
            return Ok(());
        }
    }

    let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

pub fn to_dashed_uuid(s: &str) -> String {
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

pub fn to_undashed_uuid(s: &str) -> String {
    s.replace('-', "")
}

async fn find_skin_path(uuid_or_name: &str) -> Option<PathBuf> {
    let dashed = to_dashed_uuid(uuid_or_name);
    let undashed = to_undashed_uuid(uuid_or_name);

    if let Ok(p) = skin_file_path(&dashed)
        && p.exists()
    {
        return Some(p);
    }
    if let Ok(p) = skin_file_path(&undashed)
        && p.exists()
    {
        return Some(p);
    }
    if let Ok(p) = skin_file_path(uuid_or_name)
        && p.exists()
    {
        return Some(p);
    }

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

async fn resolve_username(uuid_input: &str) -> String {
    let dashed = to_dashed_uuid(uuid_input);
    let undashed = to_undashed_uuid(uuid_input);

    if let Some(accounts) = REGISTERED_ACCOUNTS.get() {
        let read = accounts.read();
        if let Some(reg) = read.get(&dashed).or_else(|| read.get(&undashed)) {
            return reg.username.clone();
        }
    }

    // Try reading from auth.json
    if let Ok(auth_p) = oneclient_common::paths::auth_file()
        && let Ok(content) = polyio::read_to_string(&auth_p).await
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(users) = val.get("users").and_then(|u| u.as_object())
    {
        for (id, user) in users {
            if (id == &dashed || id.replace('-', "") == undashed)
                && let Some(name) = user.get("username").and_then(|n| n.as_str())
            {
                return name.to_string();
            }
        }
    }

    "Player".to_string()
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

    let username = resolve_username(uuid_input).await;
    let skin_url = format!("http://127.0.0.1:{port}/textures/{undashed}.png");

    let model_json = if is_slim {
        r#","metadata":{"model":"slim"}"#
    } else {
        ""
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let texture_payload = format!(
        r#"{{"timestamp":{timestamp},"profileId":"{undashed}","profileName":"{username}","textures":{{"SKIN":{{"url":"{skin_url}"{model_json}}}}}}}"#
    );

    let encoded_textures =
        base64::engine::general_purpose::STANDARD.encode(texture_payload.as_bytes());

    let profile_json = if let Some(sig) = sign_sha1_with_rsa(encoded_textures.as_bytes()) {
        format!(
            r#"{{"id":"{undashed}","name":"{username}","properties":[{{"name":"textures","value":"{encoded_textures}","signature":"{sig}"}}]}}"#
        )
    } else {
        format!(
            r#"{{"id":"{undashed}","name":"{username}","properties":[{{"name":"textures","value":"{encoded_textures}"}}]}}"#
        )
    };

    Some(profile_json)
}

/// Builds the `--userProperties` JSON map for launch arguments with signed textures.
pub async fn build_user_properties(uuid_input: &str, port: u16) -> Option<String> {
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

    let username = resolve_username(uuid_input).await;
    let skin_url = format!("http://127.0.0.1:{port}/textures/{undashed}.png");

    let model_json = if is_slim {
        r#","metadata":{"model":"slim"}"#
    } else {
        ""
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let texture_payload = format!(
        r#"{{"timestamp":{timestamp},"profileId":"{undashed}","profileName":"{username}","textures":{{"SKIN":{{"url":"{skin_url}"{model_json}}}}}}}"#
    );

    let encoded_textures =
        base64::engine::general_purpose::STANDARD.encode(texture_payload.as_bytes());

    let prop_json = if let Some(sig) = sign_sha1_with_rsa(encoded_textures.as_bytes()) {
        format!(
            r#"{{"textures":[{{"name":"textures","value":"{encoded_textures}","signature":"{sig}"}}]}}"#
        )
    } else {
        format!(
            r#"{{"textures":[{{"name":"textures","value":"{encoded_textures}"}}]}}"#
        )
    };

    Some(prop_json)
}

async fn fetch_mojang_profile(uuid_str: &str) -> Option<String> {
    let undashed = to_undashed_uuid(uuid_str);
    if undashed.len() != 32 {
        return None;
    }

    let cache = MOJANG_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    {
        let read = cache.read();
        if let Some((resp, timestamp)) = read.get(&undashed)
            && timestamp.elapsed() < Duration::from_secs(600)
        {
            return resp.clone();
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .ok()?;

    let url = format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{undashed}?unsigned=false"
    );
    let result = match client.get(&url).send().await {
        Ok(res) if res.status() == reqwest::StatusCode::OK => match res.text().await {
            Ok(text) if !text.trim().is_empty() => Some(text),
            _ => None,
        },
        _ => None,
    };

    let mut write = cache.write();
    write.insert(undashed, (result.clone(), Instant::now()));

    result
}

/// Synchronizes custom skin to various mod directory layouts (CustomSkinLoader, OfflineSkins, etc.)
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

        if let Ok(shared) = shared_minecraft_dir()
            && shared != game_dir
        {
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

/// Automatically writes CustomSkinLoader configuration so any modded instance
/// resolves the local custom skin cleanly and fetches other multiplayer skins.
pub async fn write_custom_skin_loader_config(csl_dir: &Path) {
    let config_path = csl_dir.join("CustomSkinLoader.json");
    let content = r#"{
  "version": "14.14",
  "loadlist": [
    {
      "name": "LocalSkin",
      "type": "CustomSkinLoader",
      "root": "CustomSkinLoader/skins/{USERNAME}.png"
    },
    {
      "name": "Mojang",
      "type": "Mojang"
    },
    {
      "name": "Ely.by",
      "type": "ElyBy"
    },
    {
      "name": "SkinRestorer",
      "type": "CustomSkinAPI",
      "root": "https://api.skinrestorer.net/v1/profile/{USERNAME}"
    }
  ]
}"#;
    let _ = polyio::create_dir_all(csl_dir).await;
    let _ = polyio::write(&config_path, content.as_bytes()).await;
}

/// Fully cleans up legacy jar patching and resource packs from previous versions.
/// Restores client.jar from pristine backup (.orig) and strips the old skin resource pack
/// from options.txt so no other players are ever rendered with the user's skin!
pub async fn cleanup_legacy_skin_injection(game_dir: &Path, client_jar: &Path) {
    // 1. Restore client.jar if a .orig backup exists
    let orig_jar = client_jar.with_extension("jar.orig");
    if orig_jar.exists() {
        if let Err(e) = tokio::fs::copy(&orig_jar, client_jar).await {
            tracing::warn!(error = %e, "failed to restore pristine client jar from backup");
        } else {
            tracing::info!(jar = %client_jar.display(), "restored pristine vanilla client jar");
            let _ = tokio::fs::remove_file(&orig_jar).await;
        }
    }

    // 2. Remove legacy resource pack directories and zip files
    let resourcepacks = game_dir.join("resourcepacks");
    let pack_dir = resourcepacks.join("onelauncher_skin");
    let pack_zip = resourcepacks.join("onelauncher_skin.zip");

    if pack_dir.exists() {
        let _ = polyio::remove_dir_all(&pack_dir).await;
    }
    if pack_zip.exists() {
        let _ = polyio::remove_file(&pack_zip).await;
    }

    // 3. Remove onelauncher_skin from options.txt
    let options_txt = game_dir.join("options.txt");
    if options_txt.exists() {
        strip_resource_pack_from_options(&options_txt, "onelauncher_skin").await;
    }
}

async fn strip_resource_pack_from_options(options_txt: &Path, pack_name: &str) {
    let file_dir_entry = format!("file/{pack_name}");
    let file_zip_entry = format!("file/{pack_name}.zip");

    let Ok(content) = polyio::read_to_string(options_txt).await else {
        return;
    };

    let mut modified = false;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    for line in &mut lines {
        if line.starts_with("resourcePacks:") {
            let raw = line.trim_start_matches("resourcePacks:").trim();
            if let Ok(mut packs) = serde_json::from_str::<Vec<String>>(raw) {
                let initial_len = packs.len();
                packs.retain(|p| {
                    p != &file_dir_entry
                        && p != &file_zip_entry
                        && p != &format!("\"{file_dir_entry}\"")
                        && p != &format!("\"{file_zip_entry}\"")
                });
                if packs.len() != initial_len {
                    modified = true;
                    *line = format!(
                        "resourcePacks:{}",
                        serde_json::to_string(&packs).unwrap_or_else(|_| "[\"vanilla\"]".to_string())
                    );
                }
            }
        } else if line.starts_with("incompatibleResourcePacks:") {
            let raw = line.trim_start_matches("incompatibleResourcePacks:").trim();
            if let Ok(mut packs) = serde_json::from_str::<Vec<String>>(raw) {
                let initial_len = packs.len();
                packs.retain(|p| {
                    p != &file_dir_entry
                        && p != &file_zip_entry
                        && p != &format!("\"{file_dir_entry}\"")
                        && p != &format!("\"{file_zip_entry}\"")
                });
                if packs.len() != initial_len {
                    modified = true;
                    *line = format!(
                        "incompatibleResourcePacks:{}",
                        serde_json::to_string(&packs).unwrap_or_else(|_| "[]".to_string())
                    );
                }
            }
        }
    }

    if modified {
        let new_content = lines.join("\n") + "\n";
        let _ = polyio::write(options_txt, new_content.as_bytes()).await;
        tracing::info!(options = %options_txt.display(), "cleaned legacy skin pack from options.txt");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_conversion() {
        let dashed = "c4436573-0477-3e11-97b7-6f7ef57f354f";
        let undashed = "c443657304773e1197b76f7ef57f354f";

        assert_eq!(to_undashed_uuid(dashed), undashed);
        assert_eq!(to_dashed_uuid(undashed), dashed);
        assert_eq!(to_dashed_uuid(dashed), dashed);
        assert_eq!(to_undashed_uuid(undashed), undashed);
    }

    #[test]
    fn test_register_launch_account() {
        register_launch_account("c4436573-0477-3e11-97b7-6f7ef57f354f", "oiupoyt");
        let accounts = REGISTERED_ACCOUNTS.get().unwrap().read();
        assert!(accounts.contains_key("c4436573-0477-3e11-97b7-6f7ef57f354f"));
        assert!(accounts.contains_key("c443657304773e1197b76f7ef57f354f"));
        assert!(accounts.contains_key("oiupoyt"));
    }

    #[tokio::test]
    async fn test_prepare_authlib_injector() {
        let res = prepare_authlib_injector().await;
        assert!(res.is_ok());
        let jar_path = res.unwrap();
        assert!(jar_path.exists());
        let meta = tokio::fs::metadata(&jar_path).await.unwrap();
        assert!(meta.len() > 100_000);
    }

    #[test]
    fn test_sign_sha1_with_rsa() {
        let sig = sign_sha1_with_rsa(b"hello world test");
        assert!(sig.is_some());
        let sig_str = sig.unwrap();
        assert!(!sig_str.is_empty());
    }

    #[tokio::test]
    async fn test_skin_server_http_endpoints() {
        let port = ensure_skin_server().await.expect("skin server starts");
        let client = reqwest::Client::new();

        // 1. Test GET /
        let res = client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .expect("GET / succeeds");
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let root_json: serde_json::Value = res.json().await.expect("valid JSON");
        assert!(root_json.get("signaturePublickey").is_some());
        assert!(root_json.get("skinDomains").is_some());

        // 2. Test GET /publickeys
        let res = client
            .get(format!("http://127.0.0.1:{port}/publickeys"))
            .send()
            .await
            .expect("GET /publickeys succeeds");
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let pk_json: serde_json::Value = res.json().await.expect("valid JSON");
        assert!(pk_json.get("profilePropertyKeys").is_some());

        // 3. Test GET /sessionserver/session/minecraft/profile/<unknown_uuid>
        // Unknown player must return 204 No Content so other multiplayer players never get user skin!
        let res = client
            .get(format!("http://127.0.0.1:{port}/sessionserver/session/minecraft/profile/00000000000000000000000000000000"))
            .send()
            .await
            .expect("GET profile succeeds");
        assert_eq!(res.status(), reqwest::StatusCode::NO_CONTENT);

        // 4. Test registered user with a skin
        let test_uuid = "12345678-1234-1234-1234-123456789abc";
        let test_undashed = "12345678123412341234123456789abc";
        let test_user = "TestPlayer";
        register_launch_account(test_uuid, test_user);

        if let Ok(dir) = skins_dir() {
            let _ = polyio::create_dir_all(&dir).await;
            let skin_p = dir.join(format!("{test_uuid}.png"));
            // 1x1 png or dummy png
            let dummy_png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15c4\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82";
            let _ = polyio::write(&skin_p, dummy_png).await;

            let res = client
                .get(format!("http://127.0.0.1:{port}/sessionserver/session/minecraft/profile/{test_undashed}"))
                .send()
                .await
                .expect("GET registered profile succeeds");
            assert_eq!(res.status(), reqwest::StatusCode::OK);
            let prof: serde_json::Value = res.json().await.expect("valid profile JSON");
            assert_eq!(prof["name"], test_user);
            let props = prof["properties"].as_array().expect("properties array");
            assert!(!props.is_empty());
            assert_eq!(props[0]["name"], "textures");
            assert!(props[0]["value"].as_str().is_some());
            assert!(props[0]["signature"].as_str().is_some());

            // Test GET /textures/<undashed>.png
            let res = client
                .get(format!("http://127.0.0.1:{port}/textures/{test_undashed}.png"))
                .send()
                .await
                .expect("GET texture succeeds");
            assert_eq!(res.status(), reqwest::StatusCode::OK);
            assert_eq!(res.headers()["content-type"], "image/png");

            let _ = polyio::remove_file(&skin_p).await;
        }
    }
}
