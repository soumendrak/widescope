#!/usr/bin/env bash
set -uo pipefail
port="${1:?usage: launch.sh <port>}"
dir="/tmp/verify-widescope/$port"
here="$(cd "$(dirname "$0")" && pwd)"
cd "$(git rev-parse --show-toplevel)"
mkdir -p "$dir"
exec 9>"$dir.lock"
flock -n 9 && [ "$(stat -c %i "$dir.lock" 2>/dev/null)" = "$(stat -L -c %i /proc/$$/fd/9)" ] || { echo "another launch.sh holds port $port; nothing started"; exit 1; }
if [ -n "$(ss -ltnH "sport = :$port")" ]; then
  echo "port $port is taken; nothing started"
  exit 1
fi
rm -f "$dir/server.pid"
old=( "$dir"/*.png "$dir"/*-drive.json )
if [ -e "${old[0]}" ]; then
  archive="$dir/previous-$(date +%Y%m%d-%H%M%S)"
  mkdir -p "$archive" && mv "$dir"/*.png "$dir"/*-drive.json "$archive"/ 2>/dev/null
  echo "moved the last launch's evidence to $archive"
fi
[ -e Cargo.lock ] || cargo generate-lockfile --quiet
"$here/inputs.sh" >"$dir/build-started"
just build-wasm >"$dir/build.log" 2>&1 && (cd ui && npm run build) >>"$dir/build.log" 2>&1 || { echo "build failed, see $dir/build.log"; exit 1; }
(cd ui && setsid -f sh -c 'echo $$ >"$1/server.pid"; exec npx vite preview --host 127.0.0.1 --port "$2" --strictPort' sh "$dir" "$port" >"$dir/server.log" 2>&1 </dev/null 9>&-)
answered=""
for _ in $(seq 60); do
  curl -sf --max-time 5 "http://127.0.0.1:$port/editor/" >/dev/null && answered=yes && break
  sleep 1
done
pgid="$(cat "$dir/server.pid" 2>/dev/null)"
listener="$(ss -ltnpH "sport = :$port" | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2)"
if [ -n "$answered" ] && [ -n "$pgid" ] && [ -n "$listener" ] && [ "$(ps -o pgid= -p "$listener" | tr -d ' ')" = "$pgid" ]; then
  echo "ready: http://127.0.0.1:$port/editor/ (process group $pgid, evidence in $dir)"
  exit 0
fi
echo "preview did not start; log follows"
cat "$dir/server.log"
if [ -n "$pgid" ] && ps -o args= -p "$pgid" | grep -qF -- "vite preview --host 127.0.0.1 --port $port"; then
  kill -- -"$pgid" 2>/dev/null
fi
exit 1
