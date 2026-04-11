# Agent Browser Cloaked

A stealth browser automation CLI for AI agents that bypasses bot detection.

## Overview

This project combines:

1. **acbrowser** - A Rust CLI for browser automation (forked from Vercel's agent-browser)
2. **CloakBrowser** - A stealth Chromium binary with 48 C++ patches to bypass bot detection

## Features

- **Stealth Mode**: Bypasses FingerprintJS, Cloudflare, and other bot detection systems
- **ARM64 Support**: Works on Linux ARM64 (uses CloakBrowser's patched Chromium)
- **Smart Page Loading**: Defaults to `DomContentLoaded` instead of `Load` to handle modern SPAs
- **MCP Compatible**: Works with the Model Context Protocol for AI agent integration

## Installation

```bash
# Clone the repository
git clone <repo-url>
cd agent-browser-cloaked

# Build the CLI (requires Rust)
cd acbrowser/cli
cargo build --release

# Copy the binary
cp target/release/acbrowser ../bin/acbrowser-linux-arm64
```

## Configuration

The browser uses the CloakBrowser stealth Chromium by default. On first run, it will be downloaded automatically to `~/.cloakbrowser/`.

You can also configure the browser via:

1. **Config file**: `~/.agent-browser/config.json`
   ```json
   {
     "executablePath": "/home/user/.cloakbrowser/chromium-VERSION/chrome",
     "args": "--no-sandbox",
     "headed": true
   }
   ```

2. **Environment variable**: `AGENT_BROWSER_EXECUTABLE_PATH`

## Usage

```bash
# Open a URL
acbrowser open https://example.com --headed

# Take a snapshot
acbrowser snapshot -i

# Click an element
acbrowser click @e1

# Type text
acbrowser fill @e2 "hello@example.com"

# Get page info
acbrowser get url
acbrowser get title
```

## Project Structure

```
agent-browser-cloaked/
├── acbrowser/           # Rust CLI for browser automation
│   ├── bin/             # Compiled binaries
│   │   └── acbrowser-linux-arm64
│   ├── cli/             # Rust source code
│   │   ├── src/
│   │   ├── Cargo.toml
│   │   └── ...
│   └── README.md
├── CloakBrowser/        # Stealth Chromium binary
│   ├── js/              # Node.js package for downloading
│   ├── bin/             # Binary download scripts
│   └── README.md
└── README.md            # This file
```

## Changes from Original

- **Default wait strategy**: Changed from `WaitUntil::Load` to `WaitUntil::DomContentLoaded` to handle modern SPAs that never fully "load"
- **ARM64 support**: Uses CloakBrowser's patched Chromium for ARM64 Linux
- **Stealth features**: Includes all CloakBrowser patches for bot detection bypass

## License

See individual component licenses:
- acbrowser: MIT
- CloakBrowser: See LICENSE file
