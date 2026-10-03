#!/bin/sh
# Exit non-zero (and print why) when /healthz isn't 200. Use from cron, an uptime checker or a
# systemd timer: */5 * * * * /srv/equihash/current/deploy/check-health.sh || <alert>
URL="${1:-http://127.0.0.1:8080/healthz}"
body=$(curl -sS --max-time 10 -w '\n%{http_code}' "$URL") || { echo "healthz unreachable: $URL"; exit 2; }
code=$(printf '%s' "$body" | tail -n1)
[ "$code" = "200" ] || { echo "healthz $code: $(printf '%s' "$body" | head -n1)"; exit 1; }
