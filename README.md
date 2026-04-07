# Sorry

Open-source, self-hosted voice & text communication app. Think TeamSpeak with a modern UI.

**1 instance = 1 server.** No centralized account system, no tracking, no telemetry.

## What is Sorry?

Sorry is a communication platform designed for communities that want full control over their infrastructure. Unlike Discord, there are no "servers within servers" — each Sorry instance is a standalone server. Users connect to servers directly, and can be connected to multiple servers simultaneously.

- **Invite-only** — New users join via invite codes created by members with the right permissions. No email, no phone number.
- **Guest access** — Invite codes can be configured to allow guest users (no account required, limited permissions).
- **Admin bootstrap** — An admin account is created at first launch (configurable via env vars, or auto-generated). The admin is always `id=1` and is re-verified at every boot.

## Deploy

### One-command install

```bash
curl -fsSL https://raw.githubusercontent.com/Romain-Boudot/sorry/main/scripts/install.sh | bash
```

The script will:
1. Detect Docker or Podman
2. Ask a few questions (local or public, domain, admin credentials)
3. Generate all secrets and configuration
4. Start everything

### What gets deployed

```
    Caddy :80/:443
    ├── /*          -> sorry:3000   (HTTP + WebSocket)
    └── /livekit/*  -> livekit:7880 (LiveKit signaling)

    LiveKit :7881/tcp  (media TCP fallback)
    LiveKit :50000-60000/udp  (media RTP)
```

3 containers: **Sorry** (app), **Caddy** (reverse proxy), **LiveKit** (voice). Files are stored locally on disk.

### Deploy modes

| Mode | Description |
|---|---|
| Local | `http://localhost`, for testing on your machine |
| Public + IP | `http://YOUR_IP`, accessible from the network |
| Public + domain | `http://domain.com`, with optional HTTPS (auto TLS via Caddy) |

### After install

```bash
cd ~/sorry

# View logs
docker compose logs -f

# Stop
docker compose down

# Update (automatic)
curl -fsSL https://raw.githubusercontent.com/Romain-Boudot/sorry/main/scripts/update.sh | bash

# Update (manual)
docker compose pull && docker compose up -d
```

Configuration is in `~/sorry/.env`.

## Architecture

### Backend — Rust (Axum)

- **HTTP API** for auth, channels, messages, roles, users, invites
- **WebSocket** for real-time events (messages, presence, voice state, role changes)
  - Full state snapshot on connect/reconnect
  - Sequenced events with gap detection and auto-resync
  - Exponential backoff reconnection (1s → 30s)
- **SQLite** via `sqlx` with WAL mode
- **FTS5** full-text search index on messages
- **JWT** authentication with `argon2` password hashing
- **TOTP** two-factor authentication (optional per user)
- **Local filesystem** for file storage (uploads, avatars, server icons)
- **LiveKit** integration for voice/video (token generation, server-side force mute/deafen)

### Frontend — Vue 3 + TypeScript

- **Vite** build toolchain, **Bun** as package manager
- **Tauri v2** for desktop app (optional, web client works standalone)
- Reactive store with per-server state management (no Pinia)
- **LiveKit client SDK** for WebRTC voice/video
- **Lucide** icons

## Features

### Text
- Channels organized in collapsible groups with drag & drop reordering
- Message editing, deletion, file attachments (images, documents, archives)
- Emoji reactions with quick-react (top 3 most-used emojis)
- Message replies with preview
- @user and @role mentions
- Markdown rendering (bold, italic, code, links, tables, blockquotes)
- Link previews with OpenGraph metadata
- Full-text search per channel (FTS5, prefix matching)
- File gallery per channel (media grid + file list with lightbox viewer)
- Infinite scroll for message history

### Voice
- LiveKit-powered voice channels
- Screen sharing and camera support
- Self mute / deafen with persistent state across reconnects
- Force mute / force deafen by admins (server-side via LiveKit API)
- Speaking indicator on user avatars
- Audio device selection (microphone, speaker) with live mic test

### Users
- User avatars (upload, per-server)
- User cards with role management
- Role-colored usernames
- Online/offline user list with avatars
- Default display name and avatar for new server joins
- Guest user support via invite codes

### Permissions & Roles
- 25 bitflag permissions (Administrator, Manage Channels, Manage Roles, Ban Members, etc.)
- Role-based with channel permission overwrites (allow/deny per role per channel)
- Role hierarchy enforcement (position-based)
- Drag & drop role reordering
- Protected roles: Admin (all permissions, non-modifiable), Membre (non-deletable)

### Server management
- Server name, description, and icon (editable by admins)
- Real-time sync of server info changes via WebSocket
- Ban system with force WebSocket disconnect and in-memory lookup
- Invite system: create codes with max uses, expiration, and optional guest mode
- Moderation panel: user message audit, activity tabs, role management
- Notification preferences per channel/server (all, mentions only, nothing)

### Multi-server
- Connect to multiple servers simultaneously
- Independent WebSocket + state per server
- Server icons in the sidebar with drag & drop reordering
- Unread count badges, voice indicator
- Skeleton loading states during connection

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/)
- [Docker](https://docs.docker.com/get-docker/) or [Podman](https://podman.io/) (for LiveKit)

### Setup

```bash
# Start LiveKit
docker compose -f docker-compose.dev.yml up -d

# Install dependencies and setup database
./scripts/setup.sh

# Start server + client
./scripts/dev.sh         # browser at http://localhost:5173
./scripts/dev-tauri.sh   # desktop app (Tauri)
```

The `.env` file at the root is pre-configured for local development.

### Building the Docker image

```bash
./scripts/build-image.sh sorry
./scripts/build-image.sh sorry ghcr.io/youruser # build + push
```

## Project Structure

```
sorry/
├── crates/
│   ├── server/        # Axum HTTP + WebSocket server
│   └── shared/        # Shared types, models, permissions, events
├── client/
│   ├── src-tauri/     # Tauri desktop wrapper
│   └── src/
│       ├── api.ts          # API client + WebSocket connection
│       ├── store.ts        # Reactive state management
│       ├── voice.ts        # LiveKit room management
│       ├── permissions.ts  # Permission constants
│       ├── composables/    # Business logic (connection, messaging, events, notifications)
│       └── components/     # Vue SFCs
├── migrations/        # SQLite migrations (001-013)
├── scripts/           # Dev, build, and deploy scripts
├── Dockerfile         # Server image (multi-stage Rust + Alpine)
├── Caddyfile          # Reverse proxy config
└── docker-compose.yml # Production deployment
```

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | **Yes** | — | Secret key for JWT signing |
| `DATABASE_URL` | No | `sqlite:./data.db` | SQLite database path |
| `SERVER_NAME` | No | `Sorry Server` | Server display name |
| `ADMIN_USERNAME` | No | `admin` | Admin account username (created at first boot) |
| `ADMIN_PASSWORD` | No | *auto-generated* | Admin account password (logged at first boot if generated) |
| `BIND_ADDR` | No | `0.0.0.0:3000` | Server listen address |
| `RUST_LOG` | No | `server=debug` | Log level filter (`server=info` recommended for production) |
| `JWT_TTL_DAYS` | No | `5` | JWT token lifetime in days |
| `LIVEKIT_URL` | No | — | Public LiveKit WebSocket URL (required for voice) |
| `LIVEKIT_INTERNAL_URL` | No | `http://livekit:7880` | LiveKit API URL for server-side calls |
| `LIVEKIT_API_KEY` | No | — | LiveKit API key (required for voice) |
| `LIVEKIT_API_SECRET` | No | — | LiveKit API secret (required for voice) |
| `UPLOAD_DIR` | No | `./data/uploads` | Local directory for file storage |
| `MAX_FILE_SIZE_MB` | No | `25` | Max upload file size in MB |
| `TLS_CERT` | No | — | Path to TLS certificate (for HTTPS without reverse proxy) |
| `TLS_KEY` | No | — | Path to TLS private key |

## Tech Stack

- **Rust** — Axum, SQLx, jsonwebtoken, argon2, tokio, totp-rs
- **Vue 3** — Composition API, `<script setup>`, scoped styles
- **TypeScript**
- **SQLite** with FTS5 full-text search
- **LiveKit** for voice/video
- **Local filesystem** for file storage
- **Caddy** for reverse proxy + auto TLS
- **Tauri v2** for desktop builds

## License

Open-source. License TBD.
