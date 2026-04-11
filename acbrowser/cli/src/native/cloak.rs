//! CloakBrowser stealth Chromium support
//!
//! CloakBrowser is a patched Chromium with 48 C++ modifications to bypass
//! bot detection systems like FingerprintJS, Cloudflare, etc.

use std::io::Write;
use std::path::PathBuf;

/// Find CloakBrowser stealth Chromium binary
///
/// Checks ~/.cloakbrowser/ directory for downloaded stealth browser.
/// The binary is downloaded automatically by the CloakBrowser JS package
/// or can be installed manually.
pub fn find_cloak_browser() -> Option<PathBuf> {
    // Check HOME environment variable
    let home = std::env::var("HOME").ok()?;
    let cloak_dir = PathBuf::from(&home).join(".cloakbrowser");

    if !cloak_dir.exists() {
        // Try to download CloakBrowser automatically
        return None;
    }

    // Find the latest chromium version directory
    let entries = std::fs::read_dir(&cloak_dir).ok()?;
    let mut versions: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("chromium-"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    // Sort by version (newest first)
    versions.sort();
    versions.reverse();

    for version in versions {
        let chrome_path = cloak_dir.join(&version).join("chrome");
        if chrome_path.exists() {
            let _ = writeln!(
                std::io::stderr(),
                "✓ Using CloakBrowser stealth Chromium: {}",
                chrome_path.display()
            );
            return Some(chrome_path);
        }
    }

    None
}

/// Get instructions for installing CloakBrowser
pub fn get_install_instructions() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    format!(
        "CloakBrowser stealth Chromium not found.\n\
         \n\
         The stealth browser is required for bot detection bypass.\n\
         \n\
         Options:\n\
         1. Install via npm: npm install -g cloakbrowser\n\
         2. Or download manually to: {}/.cloakbrowser/chromium-<version>/chrome\n\
         \n\
         Visit https://cloakbrowser.dev for more information.",
        home
    )
}

/// Check if CloakBrowser is installed
pub fn is_cloakbrowser_installed() -> bool {
    find_cloak_browser().is_some()
}
