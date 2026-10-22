#!/usr/bin/env bash
set -uo pipefail
port="${1:?usage: stop.sh <port>}"
pidfile="/tmp/verify-widescope/$port/server.pid"
lock="/tmp/verify-widescope/$port.lock"
[ -e "$lock" ] && flock -n "$lock" rm -f "$lock"
[ -f "$pidfile" ] || { echo "nothing recorded for port $port"; exit 0; }
pgid="$(cat "$pidfile")"
listener="$(ss -ltnpH "sport = :$port" | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2)"
if [ -n "$listener" ] && [ "$(ps -o pgid= -p "$listener" | tr -d ' ')" = "$pgid" ]; then
  kill -- -"$pgid" && echo "stopped process group $pgid"
else
  echo "port $port is not served by process group $pgid; killed nothing"
fi
rm -f "$pidfile"
echo "evidence kept in /tmp/verify-widescope/$port"
