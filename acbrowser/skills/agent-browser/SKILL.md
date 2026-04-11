---
name: acbrowser
description: Browser automation CLI for AI agents. Use when the user needs to interact with websites, including navigating pages, filling forms, clicking buttons, taking screenshots, extracting data, testing web apps, or automating any browser task. Triggers include requests to "open a website", "fill out a form", "click a button", "take a screenshot", "scrape data from a page", "test this web app", "login to a site", "automate browser actions", or any task requiring programmatic web interaction.
allowed-tools: Bash(npx acbrowser:*), Bash(acbrowser:*)
---

# Browser Automation with acbrowser

The CLI uses Chrome/Chromium via CDP directly. Install via `npm i -g acbrowser`, `brew install acbrowser`, or `cargo install acbrowser`. Run `acbrowser install` to download Chrome. Existing Chrome, Brave, Playwright, and Puppeteer installations are detected automatically. Run `acbrowser upgrade` to update to the latest version.

## Core Workflow

Every browser automation follows this pattern:

1. **Navigate**: `acbrowser open <url>`
2. **Snapshot**: `acbrowser snapshot -i` (get element refs like `@e1`, `@e2`)
3. **Interact**: Use refs to click, fill, select
4. **Re-snapshot**: After navigation or DOM changes, get fresh refs

```bash
acbrowser open https://example.com/form
acbrowser snapshot -i
# Output: @e1 [input type="email"], @e2 [input type="password"], @e3 [button] "Submit"

acbrowser fill @e1 "user@example.com"
acbrowser fill @e2 "password123"
acbrowser click @e3
acbrowser wait 2000
acbrowser snapshot -i  # Check result
```

## Command Chaining

Commands can be chained with `&&` in a single shell invocation. The browser persists between commands via a background daemon, so chaining is safe and more efficient than separate calls.

```bash
# Chain open + snapshot in one call (open already waits for page load)
acbrowser open https://example.com && acbrowser snapshot -i

# Chain multiple interactions
acbrowser fill @e1 "user@example.com" && acbrowser fill @e2 "password123" && acbrowser click @e3

# Navigate and capture
acbrowser open https://example.com && acbrowser screenshot
```

**When to chain:** Use `&&` when you don't need to read the output of an intermediate command before proceeding (e.g., open + wait + screenshot). Run commands separately when you need to parse the output first (e.g., snapshot to discover refs, then interact using those refs).

## Handling Authentication

When automating a site that requires login, choose the approach that fits:

**Option 1: Import auth from the user's browser (fastest for one-off tasks)**

```bash
# Connect to the user's running Chrome (they're already logged in)
acbrowser --auto-connect state save ./auth.json
# Use that auth state
acbrowser --state ./auth.json open https://app.example.com/dashboard
```

State files contain session tokens in plaintext -- add to `.gitignore` and delete when no longer needed. Set `AGENT_BROWSER_ENCRYPTION_KEY` for encryption at rest.

**Option 2: Chrome profile reuse (zero setup)**

```bash
# List available Chrome profiles
acbrowser profiles

# Reuse the user's existing Chrome login state
acbrowser --profile Default open https://gmail.com
```

**Option 3: Persistent profile (for recurring tasks)**

```bash
# First run: login manually or via automation
acbrowser --profile ~/.myapp open https://app.example.com/login
# ... fill credentials, submit ...

# All future runs: already authenticated
acbrowser --profile ~/.myapp open https://app.example.com/dashboard
```

**Option 4: Session name (auto-save/restore cookies + localStorage)**

```bash
acbrowser --session-name myapp open https://app.example.com/login
# ... login flow ...
acbrowser close  # State auto-saved

# Next time: state auto-restored
acbrowser --session-name myapp open https://app.example.com/dashboard
```

**Option 5: Auth vault (credentials stored encrypted, login by name)**

```bash
echo "$PASSWORD" | acbrowser auth save myapp --url https://app.example.com/login --username user --password-stdin
acbrowser auth login myapp
```

`auth login` navigates with `load` and then waits for login form selectors to appear before filling/clicking, which is more reliable on delayed SPA login screens.

**Option 6: State file (manual save/load)**

```bash
# After logging in:
acbrowser state save ./auth.json
# In a future session:
acbrowser state load ./auth.json
acbrowser open https://app.example.com/dashboard
```

See [references/authentication.md](references/authentication.md) for OAuth, 2FA, cookie-based auth, and token refresh patterns.

## Essential Commands

```bash
# Batch: ALWAYS use batch for 2+ sequential commands. Commands run in order.
acbrowser batch "open https://example.com" "snapshot -i"
acbrowser batch "open https://example.com" "screenshot"
acbrowser batch "click @e1" "wait 1000" "screenshot"

# Navigation
acbrowser open <url>              # Navigate (aliases: goto, navigate)
acbrowser close                   # Close browser
acbrowser close --all             # Close all active sessions

# Snapshot
acbrowser snapshot -i             # Interactive elements with refs (recommended)
acbrowser snapshot -i --urls      # Include href URLs for links
acbrowser snapshot -s "#selector" # Scope to CSS selector

# Interaction (use @refs from snapshot)
acbrowser click @e1               # Click element
acbrowser click @e1 --new-tab     # Click and open in new tab
acbrowser fill @e2 "text"         # Clear and type text
acbrowser type @e2 "text"         # Type without clearing
acbrowser select @e1 "option"     # Select dropdown option
acbrowser check @e1               # Check checkbox
acbrowser press Enter             # Press key
acbrowser keyboard type "text"    # Type at current focus (no selector)
acbrowser keyboard inserttext "text"  # Insert without key events
acbrowser scroll down 500         # Scroll page
acbrowser scroll down 500 --selector "div.content"  # Scroll within a specific container

# Get information
acbrowser get text @e1            # Get element text
acbrowser get url                 # Get current URL
acbrowser get title               # Get page title
acbrowser get cdp-url             # Get CDP WebSocket URL

# Wait
acbrowser wait @e1                # Wait for element
acbrowser wait 2000               # Wait milliseconds
acbrowser wait --url "**/page"    # Wait for URL pattern
acbrowser wait --text "Welcome"   # Wait for text to appear (substring match)
acbrowser wait --load networkidle # Wait for network idle (caution: see Pitfalls)
acbrowser wait --fn "!document.body.innerText.includes('Loading...')"  # Wait for text to disappear
acbrowser wait "#spinner" --state hidden  # Wait for element to disappear

# Downloads
acbrowser download @e1 ./file.pdf          # Click element to trigger download
acbrowser wait --download ./output.zip     # Wait for any download to complete
acbrowser --download-path ./downloads open <url>  # Set default download directory

# Tab management
acbrowser tab list                         # List all open tabs
acbrowser tab new                          # Open a blank new tab
acbrowser tab new https://example.com      # Open URL in a new tab
acbrowser tab 2                            # Switch to tab by index (0-based)
acbrowser tab close                        # Close the current tab
acbrowser tab close 2                      # Close tab by index

# Network
acbrowser network requests                 # Inspect tracked requests
acbrowser network requests --type xhr,fetch  # Filter by resource type
acbrowser network requests --method POST   # Filter by HTTP method
acbrowser network requests --status 2xx    # Filter by status (200, 2xx, 400-499)
acbrowser network request <requestId>      # View full request/response detail
acbrowser network route "**/api/*" --abort  # Block matching requests
acbrowser network har start                # Start HAR recording
acbrowser network har stop ./capture.har   # Stop and save HAR file

# Viewport & Device Emulation
acbrowser set viewport 1920 1080          # Set viewport size (default: 1280x720)
acbrowser set viewport 1920 1080 2        # 2x retina (same CSS size, higher res screenshots)
acbrowser set device "iPhone 14"          # Emulate device (viewport + user agent)

# Capture
acbrowser screenshot              # Screenshot to temp dir
acbrowser screenshot --full       # Full page screenshot
acbrowser screenshot --annotate   # Annotated screenshot with numbered element labels
acbrowser screenshot --screenshot-dir ./shots  # Save to custom directory
acbrowser screenshot --screenshot-format jpeg --screenshot-quality 80
acbrowser pdf output.pdf          # Save as PDF

# Live preview / streaming
acbrowser stream enable           # Start runtime WebSocket streaming on an auto-selected port
acbrowser stream enable --port 9223  # Bind a specific localhost port
acbrowser stream status           # Inspect enabled state, port, connection, and screencasting
acbrowser stream disable          # Stop runtime streaming and remove the .stream metadata file

# Clipboard
acbrowser clipboard read                      # Read text from clipboard
acbrowser clipboard write "Hello, World!"     # Write text to clipboard
acbrowser clipboard copy                      # Copy current selection
acbrowser clipboard paste                     # Paste from clipboard

# Dialogs (alert, confirm, prompt, beforeunload)
# By default, alert and beforeunload dialogs are auto-accepted so they never block the agent.
# confirm and prompt dialogs still require explicit handling.
# Use --no-auto-dialog (or AGENT_BROWSER_NO_AUTO_DIALOG=1) to disable automatic handling.
acbrowser dialog accept              # Accept dialog
acbrowser dialog accept "my input"   # Accept prompt dialog with text
acbrowser dialog dismiss             # Dismiss/cancel dialog
acbrowser dialog status              # Check if a dialog is currently open

# Diff (compare page states)
acbrowser diff snapshot                          # Compare current vs last snapshot
acbrowser diff snapshot --baseline before.txt    # Compare current vs saved file
acbrowser diff screenshot --baseline before.png  # Visual pixel diff
acbrowser diff url <url1> <url2>                 # Compare two pages
acbrowser diff url <url1> <url2> --wait-until networkidle  # Custom wait strategy
acbrowser diff url <url1> <url2> --selector "#main"  # Scope to element

# Chat (AI natural language control)
acbrowser chat "open google.com and search for cats"  # Single-shot instruction
acbrowser chat                                        # Interactive REPL mode
acbrowser -q chat "summarize this page"               # Quiet (text only, no tool calls)
acbrowser -v chat "fill in the login form"            # Verbose (show command output)
acbrowser --model openai/gpt-4o chat "take a screenshot"  # Override model
```

## Streaming

Every session automatically starts a WebSocket stream server on an OS-assigned port. Use `acbrowser stream status` to see the bound port and connection state. Use `stream disable` to tear it down, and `stream enable --port <port>` to re-enable on a specific port.

## Batch Execution

ALWAYS use `batch` when running 2+ commands in sequence. Batch executes commands in order, so dependent commands (like navigate then screenshot) work correctly. Each quoted argument is a separate command.

```bash
# Navigate and take a snapshot
acbrowser batch "open https://example.com" "snapshot -i"

# Navigate, snapshot, and screenshot in one call
acbrowser batch "open https://example.com" "snapshot -i" "screenshot"

# Click, wait, then screenshot
acbrowser batch "click @e1" "wait 1000" "screenshot"

# With --bail to stop on first error
acbrowser batch --bail "open https://example.com" "click @e1" "screenshot"
```

Only use a single command (not batch) when you need to read the output before deciding the next command. For example, you must run `snapshot -i` as a single command when you need to read the refs to decide what to click. After reading the snapshot, batch the remaining steps.

Stdin mode is also supported for programmatic use:

```bash
echo '[["open","https://example.com"],["screenshot"]]' | acbrowser batch --json
acbrowser batch --bail < commands.json
```

## Efficiency Strategies

These patterns minimize tool calls and token usage.

**Use `--urls` to avoid re-navigation.** When you need to visit links from a page, use `snapshot -i --urls` to get all href URLs upfront. Then `open` each URL directly instead of clicking refs and navigating back.

**Snapshot once, act many times.** Never re-snapshot the same page. Extract all needed info (refs, URLs, text) from a single snapshot, then batch the remaining actions.

**Multi-page workflow (e.g. "visit N sites and screenshot each"):**

```bash
# 1. Get all URLs in one call
acbrowser batch "open https://news.ycombinator.com" "snapshot -i --urls"
# Read output to extract URLs, then visit each directly:
# 2. One batch per target site
acbrowser batch "open https://github.com/example/repo" "screenshot"
acbrowser batch "open https://example.com/article" "screenshot"
acbrowser batch "open https://other.com/page" "screenshot"
```

This approach uses 4 tool calls instead of 14+. Never go back to the listing page between visits.

## Common Patterns

### Form Submission

```bash
# Navigate and get the form structure
acbrowser batch "open https://example.com/signup" "snapshot -i"
# Read the snapshot output to identify form refs, then fill and submit
acbrowser batch "fill @e1 \"Jane Doe\"" "fill @e2 \"jane@example.com\"" "select @e3 \"California\"" "check @e4" "click @e5" "wait 2000"
```

### Authentication with Auth Vault (Recommended)

```bash
# Save credentials once (encrypted with AGENT_BROWSER_ENCRYPTION_KEY)
# Recommended: pipe password via stdin to avoid shell history exposure
echo "pass" | acbrowser auth save github --url https://github.com/login --username user --password-stdin

# Login using saved profile (LLM never sees password)
acbrowser auth login github

# List/show/delete profiles
acbrowser auth list
acbrowser auth show github
acbrowser auth delete github
```

`auth login` waits for username/password/submit selectors before interacting, with a timeout tied to the default action timeout.

### Authentication with State Persistence

```bash
# Login once and save state
acbrowser batch "open https://app.example.com/login" "snapshot -i"
# Read snapshot to find form refs, then fill and submit
acbrowser batch "fill @e1 \"$USERNAME\"" "fill @e2 \"$PASSWORD\"" "click @e3" "wait --url **/dashboard" "state save auth.json"

# Reuse in future sessions
acbrowser batch "state load auth.json" "open https://app.example.com/dashboard"
```

### Session Persistence

```bash
# Auto-save/restore cookies and localStorage across browser restarts
acbrowser --session-name myapp open https://app.example.com/login
# ... login flow ...
acbrowser close  # State auto-saved to ~/.acbrowser/sessions/

# Next time, state is auto-loaded
acbrowser --session-name myapp open https://app.example.com/dashboard

# Encrypt state at rest
export AGENT_BROWSER_ENCRYPTION_KEY=$(openssl rand -hex 32)
acbrowser --session-name secure open https://app.example.com

# Manage saved states
acbrowser state list
acbrowser state show myapp-default.json
acbrowser state clear myapp
acbrowser state clean --older-than 7
```

### Working with Iframes

Iframe content is automatically inlined in snapshots. Refs inside iframes carry frame context, so you can interact with them directly.

```bash
acbrowser batch "open https://example.com/checkout" "snapshot -i"
# @e1 [heading] "Checkout"
# @e2 [Iframe] "payment-frame"
#   @e3 [input] "Card number"
#   @e4 [input] "Expiry"
#   @e5 [button] "Pay"

# Interact directly — no frame switch needed
acbrowser batch "fill @e3 \"4111111111111111\"" "fill @e4 \"12/28\"" "click @e5"

# To scope a snapshot to one iframe:
acbrowser batch "frame @e2" "snapshot -i"
acbrowser frame main          # Return to main frame
```

### Data Extraction

```bash
acbrowser batch "open https://example.com/products" "snapshot -i"
# Read snapshot to find element refs, then extract
acbrowser get text @e5           # Get specific element text

# JSON output for parsing
acbrowser snapshot -i --json
acbrowser get text @e1 --json
```

### Parallel Sessions

```bash
acbrowser --session site1 open https://site-a.com
acbrowser --session site2 open https://site-b.com

acbrowser --session site1 snapshot -i
acbrowser --session site2 snapshot -i

acbrowser session list
```

### Connect to Existing Chrome

```bash
# Auto-discover running Chrome with remote debugging enabled
acbrowser --auto-connect open https://example.com
acbrowser --auto-connect snapshot

# Or with explicit CDP port
acbrowser --cdp 9222 snapshot
```

Auto-connect discovers Chrome via `DevToolsActivePort`, common debugging ports (9222, 9229), and falls back to a direct WebSocket connection if HTTP-based CDP discovery fails.

### Color Scheme (Dark Mode)

```bash
# Persistent dark mode via flag (applies to all pages and new tabs)
acbrowser --color-scheme dark open https://example.com

# Or via environment variable
AGENT_BROWSER_COLOR_SCHEME=dark acbrowser open https://example.com

# Or set during session (persists for subsequent commands)
acbrowser set media dark
```

### Viewport & Responsive Testing

```bash
# Set a custom viewport size (default is 1280x720)
acbrowser set viewport 1920 1080
acbrowser screenshot desktop.png

# Test mobile-width layout
acbrowser set viewport 375 812
acbrowser screenshot mobile.png

# Retina/HiDPI: same CSS layout at 2x pixel density
# Screenshots stay at logical viewport size, but content renders at higher DPI
acbrowser set viewport 1920 1080 2
acbrowser screenshot retina.png

# Device emulation (sets viewport + user agent in one step)
acbrowser set device "iPhone 14"
acbrowser screenshot device.png
```

The `scale` parameter (3rd argument) sets `window.devicePixelRatio` without changing CSS layout. Use it when testing retina rendering or capturing higher-resolution screenshots.

### Visual Browser (Debugging)

```bash
acbrowser --headed open https://example.com
acbrowser highlight @e1          # Highlight element
acbrowser inspect                # Open Chrome DevTools for the active page
acbrowser record start demo.webm # Record session
acbrowser profiler start         # Start Chrome DevTools profiling
acbrowser profiler stop trace.json # Stop and save profile (path optional)
```

Use `AGENT_BROWSER_HEADED=1` to enable headed mode via environment variable. Browser extensions work in both headed and headless mode.

### Local Files (PDFs, HTML)

```bash
# Open local files with file:// URLs
acbrowser --allow-file-access open file:///path/to/document.pdf
acbrowser --allow-file-access open file:///path/to/page.html
acbrowser screenshot output.png
```

### iOS Simulator (Mobile Safari)

```bash
# List available iOS simulators
acbrowser device list

# Launch Safari on a specific device
acbrowser -p ios --device "iPhone 16 Pro" open https://example.com

# Same workflow as desktop - snapshot, interact, re-snapshot
acbrowser -p ios snapshot -i
acbrowser -p ios tap @e1          # Tap (alias for click)
acbrowser -p ios fill @e2 "text"
acbrowser -p ios swipe up         # Mobile-specific gesture

# Take screenshot
acbrowser -p ios screenshot mobile.png

# Close session (shuts down simulator)
acbrowser -p ios close
```

**Requirements:** macOS with Xcode, Appium (`npm install -g appium && appium driver install xcuitest`)

**Real devices:** Works with physical iOS devices if pre-configured. Use `--device "<UDID>"` where UDID is from `xcrun xctrace list devices`.

## Security

All security features are opt-in. By default, acbrowser imposes no restrictions on navigation, actions, or output.

### Content Boundaries (Recommended for AI Agents)

Enable `--content-boundaries` to wrap page-sourced output in markers that help LLMs distinguish tool output from untrusted page content:

```bash
export AGENT_BROWSER_CONTENT_BOUNDARIES=1
acbrowser snapshot
# Output:
# --- AGENT_BROWSER_PAGE_CONTENT nonce=<hex> origin=https://example.com ---
# [accessibility tree]
# --- END_AGENT_BROWSER_PAGE_CONTENT nonce=<hex> ---
```

### Domain Allowlist

Restrict navigation to trusted domains. Wildcards like `*.example.com` also match the bare domain `example.com`. Sub-resource requests, WebSocket, and EventSource connections to non-allowed domains are also blocked. Include CDN domains your target pages depend on:

```bash
export AGENT_BROWSER_ALLOWED_DOMAINS="example.com,*.example.com"
acbrowser open https://example.com        # OK
acbrowser open https://malicious.com       # Blocked
```

### Action Policy

Use a policy file to gate destructive actions:

```bash
export AGENT_BROWSER_ACTION_POLICY=./policy.json
```

Example `policy.json`:

```json
{ "default": "deny", "allow": ["navigate", "snapshot", "click", "scroll", "wait", "get"] }
```

Auth vault operations (`auth login`, etc.) bypass action policy but domain allowlist still applies.

### Output Limits

Prevent context flooding from large pages:

```bash
export AGENT_BROWSER_MAX_OUTPUT=50000
```

## Diffing (Verifying Changes)

Use `diff snapshot` after performing an action to verify it had the intended effect. This compares the current accessibility tree against the last snapshot taken in the session.

```bash
# Typical workflow: snapshot -> action -> diff
acbrowser snapshot -i          # Take baseline snapshot
acbrowser click @e2            # Perform action
acbrowser diff snapshot        # See what changed (auto-compares to last snapshot)
```

For visual regression testing or monitoring:

```bash
# Save a baseline screenshot, then compare later
acbrowser screenshot baseline.png
# ... time passes or changes are made ...
acbrowser diff screenshot --baseline baseline.png

# Compare staging vs production
acbrowser diff url https://staging.example.com https://prod.example.com --screenshot
```

`diff snapshot` output uses `+` for additions and `-` for removals, similar to git diff. `diff screenshot` produces a diff image with changed pixels highlighted in red, plus a mismatch percentage.

## Timeouts and Slow Pages

The default timeout is 25 seconds. This can be overridden with the `AGENT_BROWSER_DEFAULT_TIMEOUT` environment variable (value in milliseconds).

**Important:** `open` already waits for the page `load` event before returning. In most cases, no additional wait is needed before taking a snapshot or screenshot. Only add an explicit wait when content loads asynchronously after the initial page load.

```bash
# Wait for a specific element to appear (preferred for dynamic content)
acbrowser wait "#content"
acbrowser wait @e1

# Wait a fixed duration (good default for slow SPAs)
acbrowser wait 2000

# Wait for a specific URL pattern (useful after redirects)
acbrowser wait --url "**/dashboard"

# Wait for text to appear on the page
acbrowser wait --text "Results loaded"

# Wait for a JavaScript condition
acbrowser wait --fn "document.querySelectorAll('.item').length > 0"
```

**Avoid `wait --load networkidle`** unless you are certain the site has no persistent network activity. Ad-heavy sites, sites with analytics/tracking, and sites with websockets will cause `networkidle` to hang indefinitely. Prefer `wait 2000` or `wait <selector>` instead.

## JavaScript Dialogs (alert / confirm / prompt)

When a page opens a JavaScript dialog (`alert()`, `confirm()`, or `prompt()`), it blocks all other browser commands (snapshot, screenshot, click, etc.) until the dialog is dismissed. If commands start timing out unexpectedly, check for a pending dialog:

```bash
# Check if a dialog is blocking
acbrowser dialog status

# Accept the dialog (dismiss the alert / click OK)
acbrowser dialog accept

# Accept a prompt dialog with input text
acbrowser dialog accept "my input"

# Dismiss the dialog (click Cancel)
acbrowser dialog dismiss
```

When a dialog is pending, all command responses include a `warning` field indicating the dialog type and message. In `--json` mode this appears as a `"warning"` key in the response object.

## Session Management and Cleanup

When running multiple agents or automations concurrently, always use named sessions to avoid conflicts:

```bash
# Each agent gets its own isolated session
acbrowser --session agent1 open site-a.com
acbrowser --session agent2 open site-b.com

# Check active sessions
acbrowser session list
```

Always close your browser session when done to avoid leaked processes:

```bash
acbrowser close                    # Close default session
acbrowser --session agent1 close   # Close specific session
acbrowser close --all              # Close all active sessions
```

If a previous session was not closed properly, the daemon may still be running. Use `acbrowser close` to clean it up, or `acbrowser close --all` to shut down every session at once.

To auto-shutdown the daemon after a period of inactivity (useful for ephemeral/CI environments):

```bash
AGENT_BROWSER_IDLE_TIMEOUT_MS=60000 acbrowser open example.com
```

## Ref Lifecycle (Important)

Refs (`@e1`, `@e2`, etc.) are invalidated when the page changes. Always re-snapshot after:

- Clicking links or buttons that navigate
- Form submissions
- Dynamic content loading (dropdowns, modals)

```bash
acbrowser click @e5              # Navigates to new page
acbrowser snapshot -i            # MUST re-snapshot
acbrowser click @e1              # Use new refs
```

## Annotated Screenshots (Vision Mode)

Use `--annotate` to take a screenshot with numbered labels overlaid on interactive elements. Each label `[N]` maps to ref `@eN`. This also caches refs, so you can interact with elements immediately without a separate snapshot.

```bash
acbrowser screenshot --annotate
# Output includes the image path and a legend:
#   [1] @e1 button "Submit"
#   [2] @e2 link "Home"
#   [3] @e3 textbox "Email"
acbrowser click @e2              # Click using ref from annotated screenshot
```

Use annotated screenshots when:

- The page has unlabeled icon buttons or visual-only elements
- You need to verify visual layout or styling
- Canvas or chart elements are present (invisible to text snapshots)
- You need spatial reasoning about element positions

## Semantic Locators (Alternative to Refs)

When refs are unavailable or unreliable, use semantic locators:

```bash
acbrowser find text "Sign In" click
acbrowser find label "Email" fill "user@test.com"
acbrowser find role button click --name "Submit"
acbrowser find placeholder "Search" type "query"
acbrowser find testid "submit-btn" click
```

## JavaScript Evaluation (eval)

Use `eval` to run JavaScript in the browser context. **Shell quoting can corrupt complex expressions** -- use `--stdin` or `-b` to avoid issues.

```bash
# Simple expressions work with regular quoting
acbrowser eval 'document.title'
acbrowser eval 'document.querySelectorAll("img").length'

# Complex JS: use --stdin with heredoc (RECOMMENDED)
acbrowser eval --stdin <<'EVALEOF'
JSON.stringify(
  Array.from(document.querySelectorAll("img"))
    .filter(i => !i.alt)
    .map(i => ({ src: i.src.split("/").pop(), width: i.width }))
)
EVALEOF

# Alternative: base64 encoding (avoids all shell escaping issues)
acbrowser eval -b "$(echo -n 'Array.from(document.querySelectorAll("a")).map(a => a.href)' | base64)"
```

**Why this matters:** When the shell processes your command, inner double quotes, `!` characters (history expansion), backticks, and `$()` can all corrupt the JavaScript before it reaches acbrowser. The `--stdin` and `-b` flags bypass shell interpretation entirely.

**Rules of thumb:**

- Single-line, no nested quotes -> regular `eval 'expression'` with single quotes is fine
- Nested quotes, arrow functions, template literals, or multiline -> use `eval --stdin <<'EVALEOF'`
- Programmatic/generated scripts -> use `eval -b` with base64

## Configuration File

Create `acbrowser.json` in the project root for persistent settings:

```json
{
  "headed": true,
  "proxy": "http://localhost:8080",
  "profile": "./browser-data"
}
```

Priority (lowest to highest): `~/.acbrowser/config.json` < `./acbrowser.json` < env vars < CLI flags. Use `--config <path>` or `AGENT_BROWSER_CONFIG` env var for a custom config file (exits with error if missing/invalid). All CLI options map to camelCase keys (e.g., `--executable-path` -> `"executablePath"`). Boolean flags accept `true`/`false` values (e.g., `--headed false` overrides config). Extensions from user and project configs are merged, not replaced.

## Deep-Dive Documentation

| Reference                                                            | When to Use                                               |
| -------------------------------------------------------------------- | --------------------------------------------------------- |
| [references/commands.md](references/commands.md)                     | Full command reference with all options                   |
| [references/snapshot-refs.md](references/snapshot-refs.md)           | Ref lifecycle, invalidation rules, troubleshooting        |
| [references/session-management.md](references/session-management.md) | Parallel sessions, state persistence, concurrent scraping |
| [references/authentication.md](references/authentication.md)         | Login flows, OAuth, 2FA handling, state reuse             |
| [references/video-recording.md](references/video-recording.md)       | Recording workflows for debugging and documentation       |
| [references/profiling.md](references/profiling.md)                   | Chrome DevTools profiling for performance analysis        |
| [references/proxy-support.md](references/proxy-support.md)           | Proxy configuration, geo-testing, rotating proxies        |

## Cloud Providers

Use `-p <provider>` (or `AGENT_BROWSER_PROVIDER`) to run against a cloud browser instead of launching a local Chrome instance. Supported providers: `agentcore`, `browserbase`, `browserless`, `browseruse`, `kernel`.

### AgentCore (AWS Bedrock)

```bash
# Credentials auto-resolved from env vars or AWS CLI (SSO, IAM roles, etc.)
acbrowser -p agentcore open https://example.com

# With persistent browser profile
AGENTCORE_PROFILE_ID=my-profile acbrowser -p agentcore open https://example.com

# With explicit region
AGENTCORE_REGION=eu-west-1 acbrowser -p agentcore open https://example.com
```

Set `AWS_PROFILE` to select a named AWS profile.

## Browser Engine Selection

Use `--engine` to choose a local browser engine. The default is `chrome`.

```bash
# Use Lightpanda (fast headless browser, requires separate install)
acbrowser --engine lightpanda open example.com

# Via environment variable
export AGENT_BROWSER_ENGINE=lightpanda
acbrowser open example.com

# With custom binary path
acbrowser --engine lightpanda --executable-path /path/to/lightpanda open example.com
```

Supported engines:
- `chrome` (default) -- Chrome/Chromium via CDP
- `lightpanda` -- Lightpanda headless browser via CDP (10x faster, 10x less memory than Chrome)

Lightpanda does not support `--extension`, `--profile`, `--state`, or `--allow-file-access`. Install Lightpanda from https://lightpanda.io/docs/open-source/installation.

## Observability Dashboard

The dashboard is a standalone background server that shows live browser viewports, command activity, and console output for all sessions.

```bash
# Start the dashboard server (background, port 4848)
acbrowser dashboard start

# All sessions are automatically visible in the dashboard
acbrowser open example.com

# Stop the dashboard
acbrowser dashboard stop
```

The dashboard runs independently of browser sessions on port 4848 (configurable with `--port`). All sessions automatically stream to the dashboard. Sessions can also be created from the dashboard UI with local engines or cloud providers.

### Dashboard AI Chat

The dashboard has an optional AI chat tab powered by the Vercel AI Gateway. Enable it by setting:

```bash
export AI_GATEWAY_API_KEY=gw_your_key_here
export AI_GATEWAY_MODEL=anthropic/claude-sonnet-4.6           # optional default
export AI_GATEWAY_URL=https://ai-gateway.vercel.sh           # optional default
```

The Chat tab is always visible in the dashboard. Set `AI_GATEWAY_API_KEY` to enable AI responses.

## Ready-to-Use Templates

| Template                                                                 | Description                         |
| ------------------------------------------------------------------------ | ----------------------------------- |
| [templates/form-automation.sh](templates/form-automation.sh)             | Form filling with validation        |
| [templates/authenticated-session.sh](templates/authenticated-session.sh) | Login once, reuse state             |
| [templates/capture-workflow.sh](templates/capture-workflow.sh)           | Content extraction with screenshots |

```bash
./templates/form-automation.sh https://example.com/form
./templates/authenticated-session.sh https://app.example.com/login
./templates/capture-workflow.sh https://example.com ./output
```
