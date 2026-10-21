#!/usr/bin/env bash
set -uo pipefail
port="${1:?usage: stop.sh <port>}"
pidfile="/tmp/verify-widescope/$port/server.pid"
[ -f "$pidfile" ] || { echo "nothing recorded for port $port"; exit 0; }
kill -- -"$(cat "$pidfile")" 2>/dev/null
rm -f "$pidfile"
echo "stopped; evidence kept in /tmp/verify-widescope/$port"
