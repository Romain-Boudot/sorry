#!/usr/bin/env bash
set -e

# Lance le backend + Tauri (qui démarre Vite lui-même)
echo "Starting server + Tauri..."

cargo run -p server &
SERVER_PID=$!

trap "kill $SERVER_PID 2>/dev/null" EXIT

cd client && bun tauri dev
