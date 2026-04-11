# Session Management

Multiple isolated browser sessions with state persistence and concurrent browsing.

**Related**: [authentication.md](authentication.md) for login patterns, [SKILL.md](../SKILL.md) for quick start.

## Contents

- [Named Sessions](#named-sessions)
- [Session Isolation Properties](#session-isolation-properties)
- [Session State Persistence](#session-state-persistence)
- [Common Patterns](#common-patterns)
- [Default Session](#default-session)
- [Session Cleanup](#session-cleanup)
- [Best Practices](#best-practices)

## Named Sessions

Use `--session` flag to isolate browser contexts:

```bash
# Session 1: Authentication flow
acbrowser --session auth open https://app.example.com/login

# Session 2: Public browsing (separate cookies, storage)
acbrowser --session public open https://example.com

# Commands are isolated by session
acbrowser --session auth fill @e1 "user@example.com"
acbrowser --session public get text body
```

## Session Isolation Properties

Each session has independent:
- Cookies
- LocalStorage / SessionStorage
- IndexedDB
- Cache
- Browsing history
- Open tabs

## Session State Persistence

### Save Session State

```bash
# Save cookies, storage, and auth state
acbrowser state save /path/to/auth-state.json
```

### Load Session State

```bash
# Restore saved state
acbrowser state load /path/to/auth-state.json

# Continue with authenticated session
acbrowser open https://app.example.com/dashboard
```

### State File Contents

```json
{
  "cookies": [...],
  "localStorage": {...},
  "sessionStorage": {...},
  "origins": [...]
}
```

## Common Patterns

### Authenticated Session Reuse

```bash
#!/bin/bash
# Save login state once, reuse many times

STATE_FILE="/tmp/auth-state.json"

# Check if we have saved state
if [[ -f "$STATE_FILE" ]]; then
    acbrowser state load "$STATE_FILE"
    acbrowser open https://app.example.com/dashboard
else
    # Perform login
    acbrowser open https://app.example.com/login
    acbrowser snapshot -i
    acbrowser fill @e1 "$USERNAME"
    acbrowser fill @e2 "$PASSWORD"
    acbrowser click @e3
    acbrowser wait --load networkidle

    # Save for future use
    acbrowser state save "$STATE_FILE"
fi
```

### Concurrent Scraping

```bash
#!/bin/bash
# Scrape multiple sites concurrently

# Start all sessions
acbrowser --session site1 open https://site1.com &
acbrowser --session site2 open https://site2.com &
acbrowser --session site3 open https://site3.com &
wait

# Extract from each
acbrowser --session site1 get text body > site1.txt
acbrowser --session site2 get text body > site2.txt
acbrowser --session site3 get text body > site3.txt

# Cleanup
acbrowser --session site1 close
acbrowser --session site2 close
acbrowser --session site3 close
```

### A/B Testing Sessions

```bash
# Test different user experiences
acbrowser --session variant-a open "https://app.com?variant=a"
acbrowser --session variant-b open "https://app.com?variant=b"

# Compare
acbrowser --session variant-a screenshot /tmp/variant-a.png
acbrowser --session variant-b screenshot /tmp/variant-b.png
```

## Default Session

When `--session` is omitted, commands use the default session:

```bash
# These use the same default session
acbrowser open https://example.com
acbrowser snapshot -i
acbrowser close  # Closes default session
```

## Session Cleanup

```bash
# Close specific session
acbrowser --session auth close

# List active sessions
acbrowser session list
```

## Best Practices

### 1. Name Sessions Semantically

```bash
# GOOD: Clear purpose
acbrowser --session github-auth open https://github.com
acbrowser --session docs-scrape open https://docs.example.com

# AVOID: Generic names
acbrowser --session s1 open https://github.com
```

### 2. Always Clean Up

```bash
# Close sessions when done
acbrowser --session auth close
acbrowser --session scrape close
```

### 3. Handle State Files Securely

```bash
# Don't commit state files (contain auth tokens!)
echo "*.auth-state.json" >> .gitignore

# Delete after use
rm /tmp/auth-state.json
```

### 4. Timeout Long Sessions

```bash
# Set timeout for automated scripts
timeout 60 acbrowser --session long-task get text body
```
