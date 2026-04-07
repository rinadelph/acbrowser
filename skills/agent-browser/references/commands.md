# Command Reference

Complete reference for all acbrowser commands. For quick start and common patterns, see SKILL.md.

## Navigation

```bash
acbrowser open <url>      # Navigate to URL (aliases: goto, navigate)
                              # Supports: https://, http://, file://, about:, data://
                              # Auto-prepends https:// if no protocol given
acbrowser back            # Go back
acbrowser forward         # Go forward
acbrowser reload          # Reload page
acbrowser close           # Close browser (aliases: quit, exit)
acbrowser connect 9222    # Connect to browser via CDP port
```

## Snapshot (page analysis)

```bash
acbrowser snapshot            # Full accessibility tree
acbrowser snapshot -i         # Interactive elements only (recommended)
acbrowser snapshot -c         # Compact output
acbrowser snapshot -d 3       # Limit depth to 3
acbrowser snapshot -s "#main" # Scope to CSS selector
```

## Interactions (use @refs from snapshot)

```bash
acbrowser click @e1           # Click
acbrowser click @e1 --new-tab # Click and open in new tab
acbrowser dblclick @e1        # Double-click
acbrowser focus @e1           # Focus element
acbrowser fill @e2 "text"     # Clear and type
acbrowser type @e2 "text"     # Type without clearing
acbrowser press Enter         # Press key (alias: key)
acbrowser press Control+a     # Key combination
acbrowser keydown Shift       # Hold key down
acbrowser keyup Shift         # Release key
acbrowser hover @e1           # Hover
acbrowser check @e1           # Check checkbox
acbrowser uncheck @e1         # Uncheck checkbox
acbrowser select @e1 "value"  # Select dropdown option
acbrowser select @e1 "a" "b"  # Select multiple options
acbrowser scroll down 500     # Scroll page (default: down 300px)
acbrowser scrollintoview @e1  # Scroll element into view (alias: scrollinto)
acbrowser drag @e1 @e2        # Drag and drop
acbrowser upload @e1 file.pdf # Upload files
```

## Get Information

```bash
acbrowser get text @e1        # Get element text
acbrowser get html @e1        # Get innerHTML
acbrowser get value @e1       # Get input value
acbrowser get attr @e1 href   # Get attribute
acbrowser get title           # Get page title
acbrowser get url             # Get current URL
acbrowser get cdp-url         # Get CDP WebSocket URL
acbrowser get count ".item"   # Count matching elements
acbrowser get box @e1         # Get bounding box
acbrowser get styles @e1      # Get computed styles (font, color, bg, etc.)
```

## Check State

```bash
acbrowser is visible @e1      # Check if visible
acbrowser is enabled @e1      # Check if enabled
acbrowser is checked @e1      # Check if checked
```

## Screenshots and PDF

```bash
acbrowser screenshot          # Save to temporary directory
acbrowser screenshot path.png # Save to specific path
acbrowser screenshot --full   # Full page
acbrowser pdf output.pdf      # Save as PDF
```

## Video Recording

```bash
acbrowser record start ./demo.webm    # Start recording
acbrowser click @e1                   # Perform actions
acbrowser record stop                 # Stop and save video
acbrowser record restart ./take2.webm # Stop current + start new
```

## Wait

```bash
acbrowser wait @e1                     # Wait for element
acbrowser wait 2000                    # Wait milliseconds
acbrowser wait --text "Success"        # Wait for text (or -t)
acbrowser wait --url "**/dashboard"    # Wait for URL pattern (or -u)
acbrowser wait --load networkidle      # Wait for network idle (or -l)
acbrowser wait --fn "window.ready"     # Wait for JS condition (or -f)
```

## Mouse Control

```bash
acbrowser mouse move 100 200      # Move mouse
acbrowser mouse down left         # Press button
acbrowser mouse up left           # Release button
acbrowser mouse wheel 100         # Scroll wheel
```

## Semantic Locators (alternative to refs)

```bash
acbrowser find role button click --name "Submit"
acbrowser find text "Sign In" click
acbrowser find text "Sign In" click --exact      # Exact match only
acbrowser find label "Email" fill "user@test.com"
acbrowser find placeholder "Search" type "query"
acbrowser find alt "Logo" click
acbrowser find title "Close" click
acbrowser find testid "submit-btn" click
acbrowser find first ".item" click
acbrowser find last ".item" click
acbrowser find nth 2 "a" hover
```

## Browser Settings

```bash
acbrowser set viewport 1920 1080          # Set viewport size
acbrowser set viewport 1920 1080 2        # 2x retina (same CSS size, higher res screenshots)
acbrowser set device "iPhone 14"          # Emulate device
acbrowser set geo 37.7749 -122.4194       # Set geolocation (alias: geolocation)
acbrowser set offline on                  # Toggle offline mode
acbrowser set headers '{"X-Key":"v"}'     # Extra HTTP headers
acbrowser set credentials user pass       # HTTP basic auth (alias: auth)
acbrowser set media dark                  # Emulate color scheme
acbrowser set media light reduced-motion  # Light mode + reduced motion
```

## Cookies and Storage

```bash
acbrowser cookies                     # Get all cookies
acbrowser cookies set name value      # Set cookie
acbrowser cookies clear               # Clear cookies
acbrowser storage local               # Get all localStorage
acbrowser storage local key           # Get specific key
acbrowser storage local set k v       # Set value
acbrowser storage local clear         # Clear all
```

## Network

```bash
acbrowser network route <url>              # Intercept requests
acbrowser network route <url> --abort      # Block requests
acbrowser network route <url> --body '{}'  # Mock response
acbrowser network unroute [url]            # Remove routes
acbrowser network requests                 # View tracked requests
acbrowser network requests --filter api    # Filter requests
```

## Tabs and Windows

```bash
acbrowser tab                 # List tabs
acbrowser tab new [url]       # New tab
acbrowser tab 2               # Switch to tab by index
acbrowser tab close           # Close current tab
acbrowser tab close 2         # Close tab by index
acbrowser window new          # New window
```

## Frames

```bash
acbrowser frame "#iframe"     # Switch to iframe by CSS selector
acbrowser frame @e3           # Switch to iframe by element ref
acbrowser frame main          # Back to main frame
```

### Iframe support

Iframes are detected automatically during snapshots. When the main-frame snapshot runs, `Iframe` nodes are resolved and their content is inlined beneath the iframe element in the output (one level of nesting; iframes within iframes are not expanded).

```bash
acbrowser snapshot -i
# @e3 [Iframe] "payment-frame"
#   @e4 [input] "Card number"
#   @e5 [button] "Pay"

# Interact directly — refs inside iframes already work
acbrowser fill @e4 "4111111111111111"
acbrowser click @e5

# Or switch frame context for scoped snapshots
acbrowser frame @e3               # Switch using element ref
acbrowser snapshot -i             # Snapshot scoped to that iframe
acbrowser frame main              # Return to main frame
```

The `frame` command accepts:
- **Element refs** — `frame @e3` resolves the ref to an iframe element
- **CSS selectors** — `frame "#payment-iframe"` finds the iframe by selector
- **Frame name/URL** — matches against the browser's frame tree

## Dialogs

By default, `alert` and `beforeunload` dialogs are automatically accepted so they never block the agent. `confirm` and `prompt` dialogs still require explicit handling. Use `--no-auto-dialog` to disable this behavior.

```bash
acbrowser dialog accept [text]  # Accept dialog
acbrowser dialog dismiss        # Dismiss dialog
acbrowser dialog status         # Check if a dialog is currently open
```

## JavaScript

```bash
acbrowser eval "document.title"          # Simple expressions only
acbrowser eval -b "<base64>"             # Any JavaScript (base64 encoded)
acbrowser eval --stdin                   # Read script from stdin
```

Use `-b`/`--base64` or `--stdin` for reliable execution. Shell escaping with nested quotes and special characters is error-prone.

```bash
# Base64 encode your script, then:
acbrowser eval -b "ZG9jdW1lbnQucXVlcnlTZWxlY3RvcignW3NyYyo9Il9uZXh0Il0nKQ=="

# Or use stdin with heredoc for multiline scripts:
cat <<'EOF' | acbrowser eval --stdin
const links = document.querySelectorAll('a');
Array.from(links).map(a => a.href);
EOF
```

## State Management

```bash
acbrowser state save auth.json    # Save cookies, storage, auth state
acbrowser state load auth.json    # Restore saved state
```

## Global Options

```bash
acbrowser --session <name> ...    # Isolated browser session
acbrowser --json ...              # JSON output for parsing
acbrowser --headed ...            # Show browser window (not headless)
acbrowser --full ...              # Full page screenshot (-f)
acbrowser --cdp <port> ...        # Connect via Chrome DevTools Protocol
acbrowser -p <provider> ...       # Cloud browser provider (--provider)
acbrowser --proxy <url> ...       # Use proxy server
acbrowser --proxy-bypass <hosts>  # Hosts to bypass proxy
acbrowser --headers <json> ...    # HTTP headers scoped to URL's origin
acbrowser --executable-path <p>   # Custom browser executable
acbrowser --extension <path> ...  # Load browser extension (repeatable)
acbrowser --ignore-https-errors   # Ignore SSL certificate errors
acbrowser --help                  # Show help (-h)
acbrowser --version               # Show version (-V)
acbrowser <command> --help        # Show detailed help for a command
```

## Debugging

```bash
acbrowser --headed open example.com   # Show browser window
acbrowser --cdp 9222 snapshot         # Connect via CDP port
acbrowser connect 9222                # Alternative: connect command
acbrowser console                     # View console messages
acbrowser console --clear             # Clear console
acbrowser errors                      # View page errors
acbrowser errors --clear              # Clear errors
acbrowser highlight @e1               # Highlight element
acbrowser inspect                     # Open Chrome DevTools for this session
acbrowser trace start                 # Start recording trace
acbrowser trace stop trace.zip        # Stop and save trace
acbrowser profiler start              # Start Chrome DevTools profiling
acbrowser profiler stop trace.json    # Stop and save profile
```

## Environment Variables

```bash
AGENT_BROWSER_SESSION="mysession"            # Default session name
AGENT_BROWSER_EXECUTABLE_PATH="/path/chrome" # Custom browser path
AGENT_BROWSER_EXTENSIONS="/ext1,/ext2"       # Comma-separated extension paths
AGENT_BROWSER_PROVIDER="browserbase"         # Cloud browser provider
AGENT_BROWSER_STREAM_PORT="9223"             # Override WebSocket streaming port (default: OS-assigned)
AGENT_BROWSER_HOME="/path/to/acbrowser"  # Custom install location
```
