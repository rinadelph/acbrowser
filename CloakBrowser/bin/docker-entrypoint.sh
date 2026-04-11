#!/bin/bash
# Start Xvfb for headed mode (Turnstile, CAPTCHAs), then run user command
Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp &
sleep 1
exec "$@"
