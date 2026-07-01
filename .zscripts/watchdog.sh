#!/bin/bash
# Watchdog: keep vite running on port 3000
set -u

LOG=/home/z/my-project/.zscripts/dev.log
VITE=/home/z/my-project/node_modules/.bin/vite

while true; do
  if ! ss -tlnp 2>/dev/null | grep -q ":3000"; then
    echo "[$(date)] vite not running, starting..." >> "$LOG"
    cd /home/z/my-project
    "$VITE" --port 3000 --host >> "$LOG" 2>&1 &
    VPID=$!
    echo "[$(date)] vite started PID=$VPID" >> "$LOG"
    # Don't wait — if it dies, the next loop iteration will detect and restart
    sleep 5
  else
    sleep 10
  fi
done
