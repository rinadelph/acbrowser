# acbrowser

**AI-agent-friendly stealth browser CLI** - Combining the power of [acbrowser](https://github.com/vercel-labs/acbrowser) with the stealth capabilities of [CloakBrowser](https://github.com/CloakHQ/CloakBrowser).

A fast native Rust CLI for browser automation that can bypass advanced bot detection including FingerprintJS and Cloudflare Turnstile.

## Why acbrowser?

- **Fast Native Rust CLI** - No Playwright or Node.js runtime required for the daemon
- **Stealth Mode** - Uses CloakBrowser's patched Chromium engine to bypass bot detection
- **AI-Agent Friendly** - Designed specifically for AI agents with accessibility tree snapshots and ref-based interactions
- **Upstream Sync** - Receives updates from both acbrowser and CloakBrowser upstreams

## Installation

### Global Installation (recommended)

Installs the native Rust binary:

```bash
npm install -g acbrowser
ac install  # Download Chrome from Chrome for Testing (first time only)
```

### Project Installation (local dependency)

For projects that want to pin the version in `package.json`:

```bash
npm install acbrowser
ac install
```

Then use via `package.json` scripts or by invoking `ac` directly.

### From Source

```bash
git clone https://github.com/rinadelph/acbrowser
cd acbrowser
pnpm install
pnpm build
pnpm build:native   # Requires Rust (https://rustup.rs)
pnpm link --global  # Makes ac available globally
ac install
```

### Linux Dependencies

On Linux, install system dependencies:

```bash
ac install --with-deps
```

### Updating

Upgrade to the latest version:

```bash
ac upgrade
```

## Quick Start

```bash
ac open example.com
ac snapshot                    # Get accessibility tree with refs
ac click @e2                   # Click by ref from snapshot
ac fill @e3 "test@example.com" # Fill by ref
ac get text @e1                # Get text by ref
ac screenshot page.png
ac close
```

### Traditional Selectors (also supported)

```bash
ac click "#submit"
ac fill "#email" "test@example.com"
ac find role button click --name "Submit"
```

## Stealth Mode

acbrowser integrates CloakBrowser's stealth capabilities:

- **Fingerprint Evasion** - Patches FingerprintJS detection
- **Cloudflare Bypass** - Handles Cloudflare Turnstile challenges
- **Automation Detection** - Disables `navigator.webdriver` and other automation fingerprints

To use stealth mode, ensure you have the CloakBrowser patched Chromium installed:

```bash
# Install CloakBrowser patched Chromium (coming soon)
ac --stealth install-cloak
```

## Upstream Sync Strategy

acbrowser maintains sync with both upstream projects:

```bash
# Sync from acbrowser (CLI improvements)
git fetch upstream
git merge upstream/main

# Sync from CloakBrowser (stealth binary improvements)
git fetch cloak-upstream
# Manually integrate stealth patches
```

## Credits

- **[acbrowser](https://github.com/vercel-labs/acbrowser)** - The original fast browser automation CLI for AI agents
- **[CloakBrowser](https://github.com/CloakHQ/CloakBrowser)** - Stealth Chromium engine with bot detection bypass

## License

Apache-2.0 - See [LICENSE](LICENSE) for details.

## Commands

### Core Commands

```bash
ac open <url>                Navigate to a URL (adds https:// automatically)
ac snapshot                  Get the accessibility tree with element refs
ac snapshot -i               Only interactive elements (buttons, inputs, links)
ac click <sel>               Click an element (CSS selector or @ref from snapshot)
ac fill <sel> <text>         Clear and fill an element with text
ac type <sel> <text>         Type text into an element (appends to existing value)
ac press <key>               Press a key: Enter, Tab, Escape, Control+a, etc.
ac screenshot [path]         Take a screenshot (prints base64 if no path given)
ac close                     Close the current browser session
```

### Navigation

```bash
ac back                      Go back in history
ac forward                   Go forward in history
ac reload                    Reload the current page
```

### Get Info

```bash
ac get text <sel>            Visible text content of an element
ac get html <sel>            Outer HTML of an element
ac get value <sel>           Current value of an input/textarea
ac get url                   Current URL
ac get title                 Page title
```

### Find Elements

```bash
ac find role <role> click    Find by ARIA role and perform action
ac find text <text> click    Find by visible text
ac find label <lbl> fill <v> Find by label text and fill
```

### Browser Settings

```bash
ac set viewport <w> <h>      Set viewport size in pixels
ac set device <name>         Emulate a device (e.g. "iPhone 15", "Pixel 7")
```

### Sessions

```bash
ac session                   Show current session name
ac session list              List all active sessions
```

## Requirements

- **Chrome** - Run `ac install` to download Chrome from [Chrome for Testing](https://developer.chrome.com/blog/chrome-for-testing/) (Google's official automation channel). Existing Chrome, Brave, Playwright, and Puppeteer installations are detected automatically.
- **Rust** - Only needed when building from source.

## Contributing

Contributions welcome! Please see the [original acbrowser repository](https://github.com/vercel-labs/acbrowser) for the base contribution guidelines.

---

**Note**: This is a fork of acbrowser with CloakBrowser stealth integration. For the original project, see [vercel-labs/acbrowser](https://github.com/vercel-labs/acbrowser).
