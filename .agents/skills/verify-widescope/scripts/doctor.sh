#!/usr/bin/env bash
set -uo pipefail
port="${1:?usage: doctor.sh <port>}"
dir="/tmp/verify-widescope/$port"
cd "$(git rev-parse --show-toplevel)"
status=0
report() { if [ "$2" = ok ]; then echo "ok   $1"; else echo "FAIL $1: $2"; status=1; fi; }
pgid="$(cat "$dir/server.pid" 2>/dev/null)"
listener="$(ss -ltnpH "sport = :$port" | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2)"
[ -n "$pgid" ] && [ -n "$listener" ] && [ "$(ps -o pgid= -p "$listener" | tr -d ' ')" = "$pgid" ] && report "port $port owned by process group $pgid" ok || report "port $port" "not served by the preview launch.sh started"
html="$(curl -sf --max-time 5 "http://127.0.0.1:$port/editor/")" && report "GET /editor/" ok || report "GET /editor/" "no 200 response"
wasm="$(ls ui/dist/assets/*.wasm 2>/dev/null | head -1)"
[ -n "$wasm" ] && cmp -s "$wasm" crates/widescope-core/pkg/widescope_core_bg.wasm && report "ui/dist has this worktree's WASM build" ok || report "ui/dist WASM" "stale or missing; stop, then launch.sh again"
edited="$(find ui/src ui/public ui/index.html ui/vite.config.ts crates/*/src crates/*/Cargo.toml Cargo.toml Cargo.lock -type f -newer "$dir/build-started" 2>/dev/null | head -1)"
[ -f "$dir/build-started" ] && [ -z "$edited" ] && report "no source edited since the build" ok || report "build" "${edited:-no build-started marker} changed after the build; stop, then launch.sh again"
[ "$status" = 0 ] && echo "DOCTOR OK" || echo "DOCTOR FAIL"
exit $status
