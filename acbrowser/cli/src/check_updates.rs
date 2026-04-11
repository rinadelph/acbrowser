//! Check for updates to acbrowser and CloakBrowser
//!
//! This module provides functionality to check for new versions of:
//! - Upstream agent-browser (Vercel)
//! - CloakBrowser stealth Chromium

use crate::color;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const AGENT_BROWSER_REPO: &str = "vercel-labs/agent-browser";
const CLOAKBROWSER_REPO: &str = "nicholasruunu/cloakbrowser";
const GITHUB_API_URL: &str = "https://api.github.com/repos";

/// Version information for a component
pub struct VersionInfo {
    pub current: String,
    pub latest: String,
    pub has_update: bool,
}

/// Fetch latest release version from GitHub
async fn fetch_github_latest_release(repo: &str) -> Result<String, String> {
    let url = format!("{}/releases/latest", repo);
    let full_url = format!("{}/{}", GITHUB_API_URL, repo);

    let client = reqwest::Client::builder()
        .user_agent("acbrowser-check-updates")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get(&full_url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status: {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    body.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string())
        .ok_or_else(|| "No tag_name field in release response".to_string())
}

/// Get current CloakBrowser version from config
fn get_current_cloakbrowser_version() -> Option<String> {
    // Check the config.ts file for the version
    let home = std::env::var("HOME").ok()?;
    let config_path = std::path::PathBuf::from(&home)
        .join(".cloakbrowser")
        .join("version.txt");

    if config_path.exists() {
        if let Ok(version) = std::fs::read_to_string(&config_path) {
            return Some(version.trim().to_string());
        }
    }

    // Check if any CloakBrowser directory exists
    let cloak_dir = std::path::PathBuf::from(&home).join(".cloakbrowser");
    if cloak_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&cloak_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("chromium-") {
                    // Extract version from directory name
                    if let Some(version) = name.strip_prefix("chromium-") {
                        return Some(version.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Check for updates to agent-browser upstream
async fn check_agent_browser_updates() -> Result<VersionInfo, String> {
    let latest = fetch_github_latest_release(AGENT_BROWSER_REPO).await?;

    // Our version is in format "0.25.3-cloaked.1", extract base version
    let current_base = CURRENT_VERSION
        .split('-')
        .next()
        .unwrap_or(CURRENT_VERSION)
        .to_string();

    let has_update = latest != current_base;

    Ok(VersionInfo {
        current: CURRENT_VERSION.to_string(),
        latest,
        has_update,
    })
}

/// Check for updates to CloakBrowser
async fn check_cloakbrowser_updates() -> Result<VersionInfo, String> {
    let latest = fetch_github_latest_release(CLOAKBROWSER_REPO).await?;

    let current = get_current_cloakbrowser_version()
        .unwrap_or_else(|| "not installed".to_string());

    let has_update = current != "not installed" && latest != current;

    Ok(VersionInfo {
        current,
        latest,
        has_update,
    })
}

/// Run the check-updates command
pub fn run_check_updates(json_mode: bool) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let agent_result = check_agent_browser_updates().await;
        let cloak_result = check_cloakbrowser_updates().await;

        if json_mode {
            let mut updates = vec![];
            let mut has_any_update = false;

            if let Ok(info) = &agent_result {
                if info.has_update {
                    has_any_update = true;
                    updates.push(serde_json::json!({
                        "component": "agent-browser",
                        "current": info.current,
                        "latest": info.latest
                    }));
                }
            }

            if let Ok(info) = &cloak_result {
                if info.has_update {
                    has_any_update = true;
                    updates.push(serde_json::json!({
                        "component": "cloakbrowser",
                        "current": info.current,
                        "latest": info.latest
                    }));
                }
            }

            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "success": true,
                    "has_updates": has_any_update,
                    "updates": updates,
                    "current_versions": {
                        "acbrowser": CURRENT_VERSION,
                        "agent_browser_base": agent_result.as_ref().map(|i| i.current.split('-').next().unwrap_or("unknown")).unwrap_or("unknown"),
                        "cloakbrowser": cloak_result.as_ref().map(|i| &i.current).unwrap_or(&"not installed".to_string()).clone()
                    }
                }))
                .unwrap()
            );
        } else {
            println!(
                "\n{} {}\n",
                color::bold("acbrowser version check"),
                color::dim(&format!("(current: {})", CURRENT_VERSION))
            );

            // Agent-browser check
            println!("{}", color::bold("agent-browser (upstream):"));
            match &agent_result {
                Ok(info) => {
                    if info.has_update {
                        println!(
                            "  {} New version available: {} → {}",
                            color::green("✓"),
                            color::dim(&info.current),
                            color::bold(&info.latest)
                        );
                        println!(
                            "  {} {}",
                            color::dim("  Release notes:"),
                            color::cyan(&format!(
                                "https://github.com/{}/releases/tag/v{}",
                                AGENT_BROWSER_REPO, info.latest
                            ))
                        );
                    } else {
                        println!(
                            "  {} Up to date: {}",
                            color::green("✓"),
                            color::dim(&info.latest)
                        );
                    }
                }
                Err(e) => {
                    println!("  {} Failed to check: {}", color::red("✗"), e);
                }
            }

            // CloakBrowser check
            println!();
            println!("{}", color::bold("CloakBrowser (stealth Chromium):"));
            match &cloak_result {
                Ok(info) => {
                    if info.current == "not installed" {
                        println!(
                            "  {} Not installed",
                            color::yellow("!")
                        );
                        println!(
                            "  {} Run {} to install",
                            color::dim("  Tip:"),
                            color::cyan("acbrowser install-stealth")
                        );
                    } else if info.has_update {
                        println!(
                            "  {} New version available: {} → {}",
                            color::green("✓"),
                            color::dim(&info.current),
                            color::bold(&info.latest)
                        );
                        println!(
                            "  {} {}",
                            color::dim("  Download:"),
                            color::cyan(&format!(
                                "https://github.com/{}/releases/tag/v{}",
                                CLOAKBROWSER_REPO, info.latest
                            ))
                        );
                    } else {
                        println!(
                            "  {} Up to date: {}",
                            color::green("✓"),
                            color::dim(&info.latest)
                        );
                    }
                }
                Err(e) => {
                    println!("  {} Failed to check: {}", color::red("✗"), e);
                }
            }

            // Summary
            println!();
            let has_updates = agent_result.as_ref().map(|i| i.has_update).unwrap_or(false)
                || cloak_result.as_ref().map(|i| i.has_update).unwrap_or(false);

            if has_updates {
                println!(
                    "{} Updates are available!",
                    color::green("✓")
                );
                println!(
                    "  {} Pull the latest changes or run {}",
                    color::dim(""),
                    color::cyan("acbrowser upgrade")
                );
            } else {
                println!(
                    "{} All components are up to date!",
                    color::green("✓")
                );
            }
            println!();
        }
    });
}
