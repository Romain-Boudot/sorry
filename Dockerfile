# Build client
FROM oven/bun:1-alpine AS client
WORKDIR /app/client
COPY client/package.json ./
RUN bun install
COPY client/ .
RUN bun run build

# Build server — step 1: cache dependencies
FROM rust:1.82-alpine AS server
RUN apk add --no-cache musl-dev sqlite-dev sqlite
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/server/Cargo.toml crates/server/Cargo.toml
COPY crates/shared/Cargo.toml crates/shared/Cargo.toml
RUN mkdir -p crates/server/src crates/shared/src \
    && echo "fn main(){}" > crates/server/src/main.rs \
    && echo "" > crates/shared/src/lib.rs \
    && cargo build --release --bin server \
    && rm -rf crates/server/src crates/shared/src

# Build server — step 2: real sources
COPY crates/ crates/
COPY migrations/ migrations/
RUN sqlite3 /tmp/build.db "" \
    && for f in migrations/*.sql; do sqlite3 /tmp/build.db < "$f"; done
ENV DATABASE_URL=sqlite:///tmp/build.db
RUN cargo build --release --bin server

# Runtime
FROM alpine:3.20
RUN apk add --no-cache sqlite-libs ca-certificates
COPY --from=server /app/target/release/server /usr/local/bin/server
COPY --from=client /app/client/dist /app/static
COPY migrations/ /app/migrations
WORKDIR /app
VOLUME /app/data
EXPOSE 3000
CMD ["server"]
