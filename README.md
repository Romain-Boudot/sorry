# Sorry

Open-source, self-hosted voice & text communication app. Think TeamSpeak with a modern UI.

**1 instance = 1 server.** No centralized account system, no tracking, no telemetry.

## What is Sorry?

Sorry is a communication platform designed for communities that want full control over their infrastructure. Unlike Discord, there are no "servers within servers" — each Sorry instance is a standalone server. Users connect to servers directly, and can be connected to multiple servers simultaneously.

- **No account required** — Users auto-register by connecting with a username and password. No email, no phone number.
- **Server password** — Servers can be protected with a shared password, required only on first connection.
- **Admin bootstrap** — An admin account is created at first launch (configurable via env vars, or auto-generated). The admin is always `id=1` and is re-verified at every boot.

## Architecture

### Backend — Rust (Axum)

- **HTTP API** for auth, channels, messages, roles, users
- **WebSocket** for real-time events (messages, presence, voice state)
- **SQLite** via `sqlx` with compile-time checked queries
- **JWT** authentication with `argon2` password hashing
- **LiveKit** integration for voice/video (token generation, room management)

### Frontend — Vue 3 + TypeScript

- **Vite** build toolchain, **Bun** as package manager
- **Tauri v2** for desktop app (optional, web client works standalone)
- Reactive store (no Pinia) with per-server state management
- **LiveKit client SDK** for WebRTC voice
- **Lucide** icons

### Shared — Rust crate

Types, models, events, and permission definitions shared between server and client (Tauri).

## Features

### Text
- Channels organized in collapsible groups
- Message grouping by author (within 5 min) with date separators
- Drag & drop channel reordering (with permission)
- Right-click context menu for channel/group creation

### Voice
- LiveKit-powered voice channels
- Self mute / deafen with UI indicators
- Force mute / force deafen by admins
- Speaking indicator (green dot + avatar border)
- Voice status broadcast to all connected users
- Persistent voice bar showing current connection across servers

### Permissions
- 25 bitflag permissions (mirrors Discord's model, simplified)
- Role-based with channel permission overwrites
- Key permissions: Administrator, Manage Channels, Manage Roles, Kick/Ban Members, Mute/Deafen Members, etc.
- Default "Membre" role with sensible defaults

### User management
- User card popup (click on any user) showing display name, username, online status, and roles
- Role assignment/removal directly from user card (with Manage Roles permission)
- Per-server display names
- Online/offline user list

### Multi-server
- Connect to multiple servers simultaneously
- Independent WebSocket + state per server
- Server mute (disconnect without removing)
- Unread count badges

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/)
- [Docker](https://docs.docker.com/get-docker/) + Docker Compose (for LiveKit and production)
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) for local development

```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

### Development

LiveKit runs in Docker, the server and client run locally.

**1. Start LiveKit**

```bash
docker compose -f docker-compose.dev.yml up
```

**2. First-time setup** (installs dependencies, sets up the database)

```bash
./scripts/setup.sh
```

**3. Start server + client**

```bash
./scripts/dev.sh         # browser at http://localhost:5173
./scripts/dev-tauri.sh   # desktop app (Tauri)
```

The API is available at `http://localhost:3000`.

The `.env` file at the root is pre-configured for local development — no changes needed.

Other useful scripts:

```bash
./scripts/reset-db.sh   # wipe and recreate the local database
```

### Deployment

Three Docker Compose configurations are available:

| File | Use case | Command |
|---|---|---|
| `docker-compose.yml` | Production with a domain + automatic TLS (Caddy) | `docker compose up -d` |
| `docker-compose.unsecure.yml` | No Caddy — plain HTTP, port configurable via `PORT` (default 80) | `docker compose -f docker-compose.unsecure.yml up -d` |
| `docker-compose.dev.yml` | Local dev (LiveKit only) | `docker compose -f docker-compose.dev.yml up` |

#### Production (with domain)

Copy `.env` and fill in the required values:

```bash
cp .env .env.production
```

Minimum required variables:

```env
DOMAIN=yourdomain.com
JWT_SECRET=a_long_random_secret
LIVEKIT_API_KEY=your_key
LIVEKIT_API_SECRET=your_secret
```

Then:

```bash
docker compose --env-file .env.production up -d
```

Caddy handles TLS automatically via Let's Encrypt. The server is available at `https://yourdomain.com`.

#### Unsecure (no domain)

For testing on a local network without a domain name:

```bash
docker compose -f docker-compose.unsecure.yml up -d
```

The server is available at `http://HOST` (or `http://HOST:PORT` if you set a custom `PORT`).

For voice to work from another machine on the network, set `LIVEKIT_URL=ws://HOST_IP:7880` before starting.

## Project Structure

```
sorry-ts-dc/
├── crates/
│   ├── server/        # Axum HTTP + WebSocket server
│   ├── shared/        # Shared types, models, permissions, events
│   └── (client/src-tauri/)  # Tauri desktop wrapper
├── client/
│   └── src/
│       ├── api.ts          # API client (all endpoints)
│       ├── store.ts        # Reactive state management
│       ├── voice.ts        # LiveKit room management
│       ├── permissions.ts  # Permission constants (mirror of shared)
│       └── components/     # Vue SFCs
├── migrations/        # SQLite migrations
└── scripts/           # Dev/setup shell scripts
```

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | Yes | — | Secret key for JWT signing |
| `DATABASE_URL` | No | `sqlite:./data.db` | SQLite database path |
| `SERVER_NAME` | No | `Sorry Server` | Server display name |
| `SERVER_PASSWORD` | No | — | Password required to register |
| `ADMIN_USERNAME` | No | `admin` | Admin account username |
| `ADMIN_PASSWORD` | No | *random* | Admin account password (logged at first boot) |
| `BIND_ADDR` | No | `0.0.0.0:3000` | Server listen address |
| `LIVEKIT_URL` | No | — | LiveKit server URL |
| `LIVEKIT_API_KEY` | No | — | LiveKit API key |
| `LIVEKIT_API_SECRET` | No | — | LiveKit API secret |

## Tech Stack

- **Rust** — Axum, SQLx, jsonwebtoken, argon2, tokio
- **Vue 3** — Composition API, `<script setup>`, scoped styles
- **TypeScript**
- **SQLite**
- **LiveKit** (Cloud or self-hosted) for voice/video
- **Tauri v2** for desktop builds
- **Bun** as package manager

## License

Open-source. License TBD.
