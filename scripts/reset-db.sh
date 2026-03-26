#!/usr/bin/env bash
set -e

echo "Resetting database..."
rm -f data.db data.db-shm data.db-wal
sqlx database create --database-url sqlite:data.db
sqlx migrate run --source migrations --database-url sqlite:data.db
echo "Database reset."
