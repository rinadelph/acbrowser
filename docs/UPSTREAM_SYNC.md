# Upstream Sync Guide

This document explains how to keep acbrowser synchronized with its upstream sources.

## Upstream Sources

acbrowser is a fork that integrates two projects:

1. **agent-browser** (vercel-labs/agent-browser)
   - CLI improvements
   - New features
   - Bug fixes
   - Remote: `upstream`

2. **CloakBrowser** (CloakHQ/CloakBrowser)
   - Stealth binary patches
   - Fingerprint evasion updates
   - Bot detection bypass improvements
   - Remote: `cloak-upstream`

## Automated Sync Checking

A GitHub Actions workflow checks both upstreams daily and creates issues when updates are available.

- **Workflow**: `.github/workflows/upstream-sync.yml`
- **Schedule**: Daily at 00:00 UTC
- **Manual trigger**: Available via GitHub Actions UI

## Manual Sync Process

### Sync from agent-browser

```bash
# Fetch latest from upstream
git fetch upstream

# Check what's new
git log HEAD..upstream/main --oneline

# Merge changes
git merge upstream/main

# Resolve conflicts if any
# Common conflict areas:
# - cli/src/flags.rs (flag definitions)
# - cli/src/native/cdp/chrome.rs (browser launch)
# - cli/Cargo.toml (dependencies)
# - package.json (npm dependencies)

# Test the build
cargo check --manifest-path cli/Cargo.toml

# Commit and push
git push origin main
```

### Sync from CloakBrowser

CloakBrowser updates are typically binary patches. Users should update their CloakBrowser installation:

```bash
# Users update via pip
pip install --upgrade cloakbrowser
```

For acbrowser code changes related to CloakBrowser:

```bash
# Fetch latest from cloak-upstream
git fetch cloak-upstream

# Check for new stealth flags or config changes
git diff HEAD..cloak-upstream/main -- cloakbrowser/config.py

# Integrate new flags into cli/src/native/cdp/cloak.rs
# Update cli/src/native/cdp/chrome.rs if needed

# Test with --stealth flag
cargo run --manifest-path cli/Cargo.toml -- open example.com --stealth
```

## Conflict Resolution Strategy

### agent-browser conflicts

1. **flags.rs**: Prefer our stealth additions, merge upstream additions
2. **chrome.rs**: Keep our cloak integration, merge upstream improvements
3. **Cargo.toml**: Merge dependencies, keep our package name
4. **package.json**: Merge dependencies, keep our package info

### CloakBrowser conflicts

1. **config.py**: Note new flags, integrate into our Rust code
2. **download.py**: Check for new binary URLs or versions

## Version Tracking

Track synced versions in the commit message:

```
Sync from upstream agent-browser (v0.25.x)
- Feature: New command
- Fix: Bug fix

CloakBrowser binary version: chromium-xxx
```

## CI/CD Integration

The upstream sync workflow:
1. Checks for new commits daily
2. Creates GitHub issues for manual review
3. Does NOT auto-merge (to prevent breaking changes)

## Best Practices

1. **Review before merging**: Always check the diff
2. **Test locally**: Run `cargo check` and manual tests
3. **Keep stealth additions**: Preserve our `--stealth` integration
4. **Document changes**: Update README.md if features change
5. **Version bumps**: Update package version if significant changes

## Troubleshooting

### Build fails after merge

```bash
# Check for new dependencies
cargo build --manifest-path cli/Cargo.toml

# Update lock file
cargo update --manifest-path cli/Cargo.lock

# Check for breaking changes in flags.rs
grep -r "struct.*Flags" cli/src/
```

### Stealth mode not working

```bash
# Check CloakBrowser binary is installed
pip show cloakbrowser

# Verify binary path
ls ~/.cloakbrowser/

# Set explicit path if needed
export CLOAKBROWSER_BINARY_PATH=~/.cloakbrowser/chromium-xxx/chrome
```
