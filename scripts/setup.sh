#!/usr/bin/env bash
set -e

echo "=== Setup ==="

# Vérifier les dépendances
command -v cargo >/dev/null 2>&1 || { echo "cargo not found. Install Rust: https://rustup.rs"; exit 1; }
command -v sqlx >/dev/null 2>&1 || { echo "Installing sqlx-cli..."; cargo install sqlx-cli --features sqlite; }
command -v bun >/dev/null 2>&1 || { echo "bun not found. Install Bun: https://bun.sh"; exit 1; }

# Créer .env si absent
if [ ! -f .env ]; then
    cp .env.example .env
    echo ".env created from .env.example — edit it if needed"
fi

# DB
echo "Setting up database..."
rm -f data.db data.db-shm data.db-wal
sqlx database create --database-url sqlite:data.db
sqlx migrate run --source migrations --database-url sqlite:data.db
echo "Database ready."

# Client
echo "Installing client dependencies..."
cd client && bun install && cd ..

echo "=== Done! ==="
echo "  cargo run -p server    → start backend"
echo "  cd client && bun dev      → start frontend"
