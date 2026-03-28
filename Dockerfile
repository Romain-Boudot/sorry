# Build client
FROM node:20-alpine AS client
WORKDIR /app/client
COPY client/package.json client/package-lock.json* ./
RUN npm install
COPY client/ .
RUN npm run build

# Build server
FROM rust:1.82-alpine AS server
RUN apk add --no-cache musl-dev sqlite-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY migrations/ migrations/
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin server

# Runtime
FROM alpine:3.20
RUN apk add --no-cache sqlite-libs ca-certificates
COPY --from=server /app/target/release/server /usr/local/bin/server
COPY --from=client /app/client/dist /app/static
COPY migrations/ /app/migrations
WORKDIR /app
EXPOSE 3000
CMD ["server"]
