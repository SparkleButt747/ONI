#!/bin/bash
# ONI TUI Screenshot Tool
# Captures the ink TUI output and renders to PNG via freeze
#
# Usage:
#   ./scripts/screenshot.sh [view] [delay] [output]
#   view:   boot | repl | mc (default: repl)
#   delay:  seconds to wait before capture (default: 5)
#   output: output file path (default: /tmp/oni-screenshot.png)

set -e

VIEW="${1:-repl}"
DELAY="${2:-5}"
OUTPUT="${3:-/tmp/oni-screenshot.png}"
ANSI_FILE="/tmp/oni-capture.ansi"

cd "$(dirname "$0")/.."

echo "Capturing ONI TUI (view=$VIEW, delay=${DELAY}s)..."

# Use script to capture TTY output with ANSI codes preserved
# The -q flag suppresses script's own messages
# We send a quit signal after the delay
(
  sleep "$DELAY"
  # Send 'q' to quit the app
  kill -INT "$SCRIPT_PID" 2>/dev/null || true
) &
TIMER_PID=$!

# Use script to run in a pseudo-TTY so ink renders properly
FORCE_COLOR=3 script -q "$ANSI_FILE" npx tsx src/demo.tsx &
SCRIPT_PID=$!

# Wait for the delay, then kill
sleep "$DELAY"
kill "$SCRIPT_PID" 2>/dev/null || true
kill "$TIMER_PID" 2>/dev/null || true
wait "$SCRIPT_PID" 2>/dev/null || true

# Clean up ANSI: remove control sequences that freeze can't handle
# Keep colour codes, strip cursor movement and screen clears
cat "$ANSI_FILE" | \
  sed 's/\x1b\[[0-9]*[ABCDK]//g' | \
  sed 's/\x1b\[2K//g' | \
  sed 's/\x1b\[\?25[hl]//g' | \
  sed 's/\x1b\[H//g' | \
  sed 's/\x1b\[2J//g' | \
  head -60 > /tmp/oni-clean.ansi

echo "Rendering to $OUTPUT..."

# Use freeze to render the ANSI output as a PNG
cat /tmp/oni-clean.ansi | freeze \
  --output "$OUTPUT" \
  --language ansi \
  --theme "dracula" \
  --window \
  --padding 20

echo "Screenshot saved to $OUTPUT"
echo "Size: $(du -h "$OUTPUT" | cut -f1)"
