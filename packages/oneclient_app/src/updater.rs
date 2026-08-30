use std::path::PathBuf;

use oneclient_events::{Choice, EventBus, Prompt};
use uuid::Uuid;

use crate::constants::GITHUB_API_LATEST_RELEASE;

pub const UPDATE_CHOICE_INSTALL: &str = "update.install";

enum UpdateAnswer {
    Install,
}

fn update_prompt(version: &str) -> Prompt<UpdateAnswer> {
    Prompt::new(
        "Update available",
        format!("OneLauncher-Cracked {version} is ready to install. Download and install it now?"),
    )
    .option(
        Choice::primary(UPDATE_CHOICE_INSTALL, "Install"),
        UpdateAnswer::Install,
    )
    .dismiss("Not now")
}

const PROGRESS_STEP: u64 = 256 * 1024;

/// SemVer parser supporting cracked revision suffixes like `2.2.3-c1`, `2.2.3.c1`, `v2.2.3-c2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrackedVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub rev: u64,
}

impl CrackedVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim().trim_start_matches(['v', 'V']);

        let (base, rev) = if let Some((base_part, rev_part)) = trimmed.split_once("-c").or_else(|| trimmed.split_once(".c")) {
            let rev_num = rev_part.parse::<u64>().ok()?;
            (base_part, rev_num)
        } else if let Some((base_part, rev_part)) = trimmed.split_once("-r").or_else(|| trimmed.split_once(".r")) {
            let rev_num = rev_part.parse::<u64>().ok()?;
            (base_part, rev_num)
        } else if let Some((base_part, rev_part)) = trimmed.split_once('-') {
            let rev_num = rev_part.trim_start_matches(['c', 'C', 'r', 'R']).parse::<u64>().unwrap_or(0);
            (base_part, rev_num)
        } else {
            (trimmed, 0)
        };

        let mut parts = base.split('.');
        let major = parts.next()?.parse::<u64>().ok()?;
        let minor = parts.next()?.parse::<u64>().ok()?;
        let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;

        Some(Self {
            major,
            minor,
            patch,
            rev,
        })
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GithubReleaseAsset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
}

impl GithubRelease {
    pub fn find_installer_asset(&self) -> Option<&GithubReleaseAsset> {
        #[cfg(target_os = "windows")]
        {
            self.assets.iter().find(|a| a.name.ends_with("_x64-setup.exe") || a.name.ends_with("-setup.exe") || a.name.ends_with(".exe"))
        }
        #[cfg(target_os = "linux")]
        {
            if std::env::var_os("APPIMAGE").is_some() {
                self.assets.iter().find(|a| a.name.ends_with(".AppImage"))
            } else {
                self.assets.iter().find(|a| a.name.ends_with(".deb") || a.name.ends_with(".AppImage") || a.name.ends_with(".rpm"))
            }
        }
        #[cfg(target_os = "macos")]
        {
            #[cfg(target_arch = "aarch64")]
            {
                self.assets.iter().find(|a| a.name.ends_with("aarch64.dmg") || a.name.ends_with("arm64.dmg") || a.name.ends_with(".dmg"))
            }
            #[cfg(not(target_arch = "aarch64"))]
            {
                self.assets.iter().find(|a| a.name.ends_with("x64.dmg") || a.name.ends_with("x86_64.dmg") || a.name.ends_with(".dmg"))
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }
}

pub fn spawn_update_check(auto_install: bool, events: EventBus) {
    tokio::spawn(async move {
        if let Err(err) = run_check(auto_install, events).await {
            tracing::warn!("update check failed: {err:#}");
        }
    });
}

/// Debug-only drives the full auto-update UX
pub fn spawn_simulated_update() {
    tokio::spawn(async move {
        if let Err(err) = run_simulated_update().await {
            tracing::warn!("simulated update failed: {err:#}");
        }
    });
}

async fn run_simulated_update() -> anyhow::Result<()> {
    const FAKE_VERSION: &str = "2.2.3-c99";
    const FAKE_TOTAL: u64 = 48 * 1024 * 1024;

    let events = crate::launcher::state()?.services.events.clone();

    if events.ask(update_prompt(FAKE_VERSION)).await?.is_none() {
        tracing::info!("user declined simulated update");
        return Ok(());
    }

    let progress_id = Uuid::new_v4();
    let label = format!("Downloading OneLauncher-Cracked {FAKE_VERSION}");

    let mut downloaded = 0u64;
    events.progress(progress_id, &label, downloaded, FAKE_TOTAL);
    while downloaded < FAKE_TOTAL {
        downloaded = (downloaded + PROGRESS_STEP * 8).min(FAKE_TOTAL);
        events.progress(progress_id, &label, downloaded, FAKE_TOTAL);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    events.finish_progress(
        progress_id,
        "Finished Downloading",
        format!("OneLauncher-Cracked {FAKE_VERSION} is ready. Restart to apply."),
    );

    Ok(())
}

async fn run_check(auto_install: bool, events: EventBus) -> anyhow::Result<()> {
    let Some(release) = check_for_github_update().await? else {
        tracing::info!("no update available");
        return Ok(());
    };

    tracing::info!("update available: {}", release.tag_name);

    if !can_self_update() {
        tracing::info!("install is not self-updatable in-place; notifying with release URL");
        events
            .notify("Update available")
            .body(format!(
                "OneLauncher-Cracked {} is available. Download from {}",
                release.tag_name, release.html_url
            ))
            .send();
        return Ok(());
    }

    if !auto_install && events.ask(update_prompt(&release.tag_name)).await?.is_none() {
        tracing::info!("user declined update {}", release.tag_name);
        return Ok(());
    }

    download_and_install_github(release, events).await
}

fn can_self_update() -> bool {
    if cfg!(debug_assertions) {
        return false;
    }

    if std::env::var_os("ONECLIENT_DISABLE_AUTOUPDATE")
        .is_some_and(|val| val.eq_ignore_ascii_case("1"))
    {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        true
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("APPIMAGE").is_some()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

async fn check_for_github_update() -> anyhow::Result<Option<GithubRelease>> {
    let client = reqwest::Client::builder()
        .user_agent("OneLauncher-Cracked-Updater")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client
        .get(GITHUB_API_LATEST_RELEASE)
        .send()
        .await?;

    if !resp.status().is_success() {
        tracing::debug!("GitHub releases API returned status: {}", resp.status());
        return Ok(None);
    }

    let release: GithubRelease = resp.json().await?;

    let current_str = env!("CARGO_PKG_VERSION");
    let current_ver = CrackedVersion::parse(current_str).unwrap_or(CrackedVersion {
        major: 2,
        minor: 2,
        patch: 3,
        rev: 1,
    });

    let remote_ver = match CrackedVersion::parse(&release.tag_name) {
        Some(v) => v,
        None => return Ok(None),
    };

    if remote_ver > current_ver {
        Ok(Some(release))
    } else {
        Ok(None)
    }
}

async fn download_and_install_github(release: GithubRelease, events: EventBus) -> anyhow::Result<()> {
    let Some(asset) = release.find_installer_asset() else {
        events
            .notify("Update available")
            .body(format!(
                "OneLauncher-Cracked {} is available at {}",
                release.tag_name, release.html_url
            ))
            .send();
        return Ok(());
    };

    let progress_id = Uuid::new_v4();
    let version = release.tag_name.clone();
    let label = format!("Downloading OneLauncher-Cracked {version}");

    events.progress(progress_id, &label, 0, asset.size);

    let client = reqwest::Client::builder()
        .user_agent("OneLauncher-Cracked-Updater")
        .build()?;

    let mut resp = client.get(&asset.browser_download_url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to download update asset: HTTP {}", resp.status());
    }

    let total_size = asset.size.max(resp.content_length().unwrap_or(0));

    #[cfg(target_os = "windows")]
    let target_file = std::env::temp_dir().join(&asset.name);

    #[cfg(target_os = "linux")]
    let (target_file, is_appimage, final_appimage_dest) = if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let dest = PathBuf::from(appimage);
        let temp_download = dest.with_extension("new_appimage");
        (temp_download, true, Some(dest))
    } else {
        (std::env::temp_dir().join(&asset.name), false, None)
    };

    #[cfg(target_os = "macos")]
    let target_file = std::env::temp_dir().join(&asset.name);

    let mut file = tokio::fs::File::create(&target_file).await?;
    let mut downloaded = 0u64;
    let mut last_sent = 0u64;

    while let Some(chunk) = resp.chunk().await? {
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        downloaded += chunk.len() as u64;

        if downloaded - last_sent >= PROGRESS_STEP || (total_size > 0 && downloaded >= total_size) {
            last_sent = downloaded;
            events.progress(progress_id, &label, downloaded, total_size);
        }
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    drop(file);

    events.progress(progress_id, &label, total_size, total_size);

    #[cfg(target_os = "windows")]
    {
        events.finish_progress(
            progress_id,
            "Launching Installer",
            format!("OneLauncher-Cracked {version} ready. Opening installer..."),
        );
        let _ = std::process::Command::new(&target_file).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        if is_appimage && let Some(dest) = final_appimage_dest {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target_file, std::fs::Permissions::from_mode(0o755));
            }
            tokio::fs::rename(&target_file, &dest).await?;
            events.finish_progress(
                progress_id,
                "Finished Updating",
                format!("OneLauncher-Cracked {version} is ready. Restart to apply."),
            );
        } else {
            events.finish_progress(
                progress_id,
                "Download Complete",
                format!("Downloaded {} to {}", asset.name, target_file.display()),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        events.finish_progress(
            progress_id,
            "Download Complete",
            format!("Downloaded {} to {}. Open to install.", asset.name, target_file.display()),
        );
        let _ = open::that(&target_file);
    }

    tracing::info!("update download and installation handling completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cracked_version_ordering() {
        let v1 = CrackedVersion::parse("2.2.3-c1").unwrap();
        let v2 = CrackedVersion::parse("2.2.3-c2").unwrap();
        let v3 = CrackedVersion::parse("v2.2.4-c1").unwrap();
        let v_base = CrackedVersion::parse("v2.2.3").unwrap();

        assert!(v2 > v1);
        assert!(v3 > v2);
        assert!(v1 > v_base);
        assert_eq!(v1, CrackedVersion { major: 2, minor: 2, patch: 3, rev: 1 });
        assert_eq!(v2, CrackedVersion { major: 2, minor: 2, patch: 3, rev: 2 });
        assert_eq!(v3, CrackedVersion { major: 2, minor: 2, patch: 4, rev: 1 });
        assert_eq!(v_base, CrackedVersion { major: 2, minor: 2, patch: 3, rev: 0 });
    }
}

