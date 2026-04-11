#!/bin/bash
# Template: Form Automation Workflow
# Purpose: Fill and submit web forms with validation
# Usage: ./form-automation.sh <form-url>
#
# This template demonstrates the snapshot-interact-verify pattern:
# 1. Navigate to form
# 2. Snapshot to get element refs
# 3. Fill fields using refs
# 4. Submit and verify result
#
# Customize: Update the refs (@e1, @e2, etc.) based on your form's snapshot output

set -euo pipefail

FORM_URL="${1:?Usage: $0 <form-url>}"

echo "Form automation: $FORM_URL"

# Step 1: Navigate to form
acbrowser open "$FORM_URL"
acbrowser wait --load networkidle

# Step 2: Snapshot to discover form elements
echo ""
echo "Form structure:"
acbrowser snapshot -i

# Step 3: Fill form fields (customize these refs based on snapshot output)
#
# Common field types:
#   acbrowser fill @e1 "John Doe"           # Text input
#   acbrowser fill @e2 "user@example.com"   # Email input
#   acbrowser fill @e3 "SecureP@ss123"      # Password input
#   acbrowser select @e4 "Option Value"     # Dropdown
#   acbrowser check @e5                     # Checkbox
#   acbrowser click @e6                     # Radio button
#   acbrowser fill @e7 "Multi-line text"   # Textarea
#   acbrowser upload @e8 /path/to/file.pdf # File upload
#
# Uncomment and modify:
# acbrowser fill @e1 "Test User"
# acbrowser fill @e2 "test@example.com"
# acbrowser click @e3  # Submit button

# Step 4: Wait for submission
# acbrowser wait --load networkidle
# acbrowser wait --url "**/success"  # Or wait for redirect

# Step 5: Verify result
echo ""
echo "Result:"
acbrowser get url
acbrowser snapshot -i

# Optional: Capture evidence
acbrowser screenshot /tmp/form-result.png
echo "Screenshot saved: /tmp/form-result.png"

# Cleanup
acbrowser close
echo "Done"
