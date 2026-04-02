# Build server
FROM rust:1.94-alpine AS builder
RUN apk add --no-cache musl-dev sqlite-dev sqlite
WORKDIR /app

COPY Cargo.lock ./
COPY crates/server/Cargo.toml crates/server/Cargo.toml
COPY crates/shared/Cargo.toml crates/shared/Cargo.toml
RUN printf '[workspace]\nmembers = ["crates/server", "crates/shared"]\nresolver = "2"\n' > Cargo.toml
RUN mkdir -p crates/server/src crates/shared/src \
    && echo "fn main(){}" > crates/server/src/main.rs \
    && echo "" > crates/shared/src/lib.rs \
    && cargo build --release --bin server \
    && rm -rf crates/server/src crates/shared/src

COPY crates/ crates/
COPY migrations/ migrations/
RUN touch crates/server/src/main.rs crates/shared/src/lib.rs
RUN sqlite3 /tmp/build.db "" \
    && for f in migrations/*.sql; do sqlite3 /tmp/build.db < "$f"; done
ENV DATABASE_URL=sqlite:///tmp/build.db
RUN cargo build --release --bin server

# Runtime
FROM alpine:3.20
RUN apk add --no-cache sqlite-libs ca-certificates
COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY migrations/ /app/migrations
WORKDIR /app
VOLUME /app/data
EXPOSE 3000
CMD ["server"]
