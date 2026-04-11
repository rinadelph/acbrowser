//! CloakBrowser stealth binary detection and management.
//!
//! CloakBrowser is a patched Chromium with source-level modifications to bypass
//! bot detection like FingerprintJS and Cloudflare Turnstile.
//!
//! Binary detection order:
//! 1. CLOAKBROWSER_BINARY_PATH environment variable
//! 2. ~/.cloakbrowser/chromium-{version}/chrome (Linux)
//! 3. ~/.cloakbrowser/chromium-{version}/Chromium.app/Contents/MacOS/Chromium (macOS)
//! 4. ~/.cloakbrowser/chromium-{version}/chrome.exe (Windows)

use std::env;
use std::path::PathBuf;

/// The default CloakBrowser cache directory
pub fn get_cloak_cache_dir() -> PathBuf {
    if let Ok(custom) = env::var("CLOAKBROWSER_CACHE_DIR") {
        PathBuf::from(custom)
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cloakbrowser")
    }
}

/// Get the platform tag for CloakBrowser binary detection
pub fn get_platform_tag() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        #[cfg(target_arch = "x86_64")]
        return "linux-x64";
        #[cfg(target_arch = "aarch64")]
        return "linux-arm64";
    }
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "x86_64")]
        return "darwin-x64";
        #[cfg(target_arch = "aarch64")]
        return "darwin-arm64";
    }
    #[cfg(target_os = "windows")]
    return "windows-x64";
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    "unknown"
}

/// Find the CloakBrowser stealth Chromium binary.
///
/// Returns the path to the CloakBrowser chrome executable if found.
///
/// Detection order:
/// 1. CLOAKBROWSER_BINARY_PATH environment variable (user override)
/// 2. Latest version in ~/.cloakbrowser/chromium-{version}/
///
/// Returns None if not found.
pub fn find_cloak_browser() -> Option<PathBuf> {
    // 1. Check for explicit override
    if let Ok(path) = env::var("CLOAKBROWSER_BINARY_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Some(p);
        }
        // Log warning if set but doesn't exist
        eprintln!("Warning: CLOAKBROWSER_BINARY_PATH set to '{}' but file not found", path);
    }
    
    // 2. Check CloakBrowser cache directory for any version
    let cache_dir = get_cloak_cache_dir();
    if cache_dir.exists() {
        // Look for chromium-* directories and use the latest
        let mut versions: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("chromium-") {
                    versions.push(name_str.into_owned());
                }
            }
        }
        
        // Sort versions and try the latest
        versions.sort();
        versions.reverse();
        
        for version in versions {
            let binary_path = get_binary_path_for_version(&version);
            if binary_path.exists() {
                return Some(binary_path);
            }
        }
    }
    
    None
}

/// Get the binary path for a specific version
fn get_binary_path_for_version(version: &str) -> PathBuf {
    let cache_dir = get_cloak_cache_dir();
    let binary_dir = cache_dir.join(version);
    
    #[cfg(target_os = "macos")]
    {
        binary_dir.join("Chromium.app/Contents/MacOS/Chromium")
    }
    #[cfg(target_os = "windows")]
    {
        binary_dir.join("chrome.exe")
    }
    #[cfg(target_os = "linux")]
    {
        binary_dir.join("chrome")
    }
}

/// Check if CloakBrowser is available
pub fn is_cloak_available() -> bool {
    find_cloak_browser().is_some()
}

/// Get instructions for installing CloakBrowser
pub fn get_install_instructions() -> String {
    format!(
        "CloakBrowser stealth Chromium not found.\n\
         \n\
         To install CloakBrowser:\n\
         \n\
         Option 1 - Install via pip (recommended):\n\
           pip install cloakbrowser\n\
         \n\
         Option 2 - Set CLOAKBROWSER_BINARY_PATH to your CloakBrowser binary:\n\
           export CLOAKBROWSER_BINARY_PATH=/path/to/your/cloak-chrome\n\
         \n\
         The binary will be detected from:\n\
           - CLOAKBROWSER_BINARY_PATH (if set)\n\
           - {}/chromium-<version>/...\n\
         \n\
         CloakBrowser provides source-level patches for:\n\
           - FingerprintJS evasion\n\
           - Cloudflare Turnstile bypass\n\
           - navigator.webdriver masking\n\
         \n\
         GitHub: https://github.com/CloakHQ/CloakBrowser",
        get_cloak_cache_dir().display()
    )
}
