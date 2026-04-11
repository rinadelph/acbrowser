#!/bin/bash
# acbrowser + CloakBrowser: AI browser agent with stealth fingerprints.
#
# acbrowser is a Node.js CLI for browser automation with session management.
# CloakBrowser provides the stealth Chromium binary.
#
# Requires: npm install -g acbrowser
#           pip install cloakbrowser (to auto-download the binary)
#
# Note: acbrowser launches Chrome itself via env vars — it can't connect
# to an existing browser via CDP. So we pass the binary path and stealth args directly.

# Get CloakBrowser binary path (auto-downloads if needed)
BINARY_PATH=$(python3 -c "from cloakbrowser.download import ensure_binary; print(ensure_binary())")

# Get stealth args from our wrapper (comma-separated for acbrowser)
STEALTH_ARGS=$(python3 -c "from cloakbrowser.config import get_default_stealth_args; print(','.join(get_default_stealth_args()))")

# Point acbrowser at CloakBrowser
export AGENT_BROWSER_EXECUTABLE_PATH="$BINARY_PATH"
export AGENT_BROWSER_ARGS="$STEALTH_ARGS"

# Open a page
acbrowser --session stealth-test open "https://example.com"

# Get page title
acbrowser --session stealth-test eval "document.title"

# Check stealth
acbrowser --session stealth-test eval "JSON.stringify({webdriver: navigator.webdriver, plugins: navigator.plugins.length, platform: navigator.platform})"
