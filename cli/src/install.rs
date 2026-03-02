use crate::color;
use crate::connection;
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};

/// Read the playwright-core version from its package.json so that the install
/// command downloads the exact Chromium revision the runtime expects.
fn resolve_playwright_version() -> Option<String> {
    let exe_dir = connection::resolve_exe_dir()?;

    let candidates = [
        exe_dir.join("../node_modules/playwright-core/package.json"),
        exe_dir.join("node_modules/playwright-core/package.json"),
    ];

    // Also check AGENT_BROWSER_HOME
    let home_candidates: Vec<PathBuf> = std::env::var("AGENT_BROWSER_HOME")
        .ok()
        .map(|h| {
            let home = PathBuf::from(h);
            vec![
                home.join("node_modules/playwright-core/package.json"),
            ]
        })
        .unwrap_or_default();

    for path in home_candidates.iter().chain(candidates.iter()) {
        if let Ok(contents) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(version) = json["version"].as_str() {
                    return Some(version.to_string());
                }
            }
        }
    }
    None
}

pub fn run_install(with_deps: bool) {
    let is_linux = cfg!(target_os = "linux");

    if is_linux {
        if with_deps {
            println!("{}", color::cyan("Installing system dependencies..."));

            let (pkg_mgr, deps) = if which_exists("apt-get") {
                let libasound = if package_exists_apt("libasound2t64") {
                    "libasound2t64"
                } else {
                    "libasound2"
                };

                (
                    "apt-get",
                    vec![
                        "libxcb-shm0",
                        "libx11-xcb1",
                        "libx11-6",
                        "libxcb1",
                        "libxext6",
                        "libxrandr2",
                        "libxcomposite1",
                        "libxcursor1",
                        "libxdamage1",
                        "libxfixes3",
                        "libxi6",
                        "libgtk-3-0",
                        "libpangocairo-1.0-0",
                        "libpango-1.0-0",
                        "libatk1.0-0",
                        "libcairo-gobject2",
                        "libcairo2",
                        "libgdk-pixbuf-2.0-0",
                        "libxrender1",
                        libasound,
                        "libfreetype6",
                        "libfontconfig1",
                        "libdbus-1-3",
                        "libnss3",
                        "libnspr4",
                        "libatk-bridge2.0-0",
                        "libdrm2",
                        "libxkbcommon0",
                        "libatspi2.0-0",
                        "libcups2",
                        "libxshmfence1",
                        "libgbm1",
                    ],
                )
            } else if which_exists("dnf") {
                (
                    "dnf",
                    vec![
                        "nss",
                        "nspr",
                        "atk",
                        "at-spi2-atk",
                        "cups-libs",
                        "libdrm",
                        "libXcomposite",
                        "libXdamage",
                        "libXrandr",
                        "mesa-libgbm",
                        "pango",
                        "alsa-lib",
                        "libxkbcommon",
                        "libxcb",
                        "libX11-xcb",
                        "libX11",
                        "libXext",
                        "libXcursor",
                        "libXfixes",
                        "libXi",
                        "gtk3",
                        "cairo-gobject",
                    ],
                )
            } else if which_exists("yum") {
                (
                    "yum",
                    vec![
                        "nss",
                        "nspr",
                        "atk",
                        "at-spi2-atk",
                        "cups-libs",
                        "libdrm",
                        "libXcomposite",
                        "libXdamage",
                        "libXrandr",
                        "mesa-libgbm",
                        "pango",
                        "alsa-lib",
                        "libxkbcommon",
                    ],
                )
            } else {
                eprintln!(
                    "{} No supported package manager found (apt-get, dnf, or yum)",
                    color::error_indicator()
                );
                exit(1);
            };

            let install_cmd = match pkg_mgr {
                "apt-get" => {
                    format!(
                        "sudo apt-get update && sudo apt-get install -y {}",
                        deps.join(" ")
                    )
                }
                _ => format!("sudo {} install -y {}", pkg_mgr, deps.join(" ")),
            };

            println!("Running: {}", install_cmd);
            let status = Command::new("sh").arg("-c").arg(&install_cmd).status();

            match status {
                Ok(s) if s.success() => {
                    println!("{} System dependencies installed", color::success_indicator())
                }
                Ok(_) => eprintln!(
                    "{} Failed to install some dependencies. You may need to run manually with sudo.",
                    color::warning_indicator()
                ),
                Err(e) => eprintln!("{} Could not run install command: {}", color::warning_indicator(), e),
            }
        } else {
            println!(
                "{} Linux detected. If browser fails to launch, run:",
                color::warning_indicator()
            );
            println!("  agent-browser install --with-deps");
            println!("  or: npx playwright install-deps chromium");
            println!();
        }
    }

    println!("{}", color::cyan("Installing Chromium browser..."));

    // Pin to the same playwright version as the bundled playwright-core so the
    // downloaded Chromium revision matches what the runtime expects.
    let pw_package = match resolve_playwright_version() {
        Some(v) => {
            println!("  Using playwright@{} (matched to bundled playwright-core)", v);
            format!("playwright@{}", v)
        }
        None => "playwright".to_string(),
    };

    #[cfg(windows)]
    let status = Command::new("cmd")
        .args(["/c", &format!("npx {} install chromium", pw_package)])
        .status();

    #[cfg(not(windows))]
    let status = Command::new("npx")
        .args([&pw_package, "install", "chromium"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!(
                "{} Chromium installed successfully",
                color::success_indicator()
            );
            if is_linux && !with_deps {
                println!();
                println!(
                    "{} If you see \"shared library\" errors when running, use:",
                    color::yellow("Note:")
                );
                println!("  agent-browser install --with-deps");
            }
        }
        Ok(_) => {
            eprintln!("{} Failed to install browser", color::error_indicator());
            if is_linux {
                println!(
                    "{} Try installing system dependencies first:",
                    color::yellow("Tip:")
                );
                println!("  agent-browser install --with-deps");
            }
            exit(1);
        }
        Err(e) => {
            eprintln!("{} Failed to run npx: {}", color::error_indicator(), e);
            eprintln!("Make sure Node.js is installed and npx is in your PATH");
            exit(1);
        }
    }
}

fn which_exists(cmd: &str) -> bool {
    #[cfg(unix)]
    {
        Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn package_exists_apt(pkg: &str) -> bool {
    Command::new("apt-cache")
        .arg("show")
        .arg(pkg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
