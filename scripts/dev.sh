#!/usr/bin/env bash
set -e

# Lance le backend et le frontend en parallèle
echo "Starting server + client..."

cargo run -p server &
SERVER_PID=$!

cd client && bun dev &
CLIENT_PID=$!

trap "kill $SERVER_PID $CLIENT_PID 2>/dev/null" EXIT

wait
