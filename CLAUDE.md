# Sorry — Self-hosted voice & text (Discord/TeamSpeak-like)

## Stack
- **Backend**: Rust (axum 0.7 + SQLite + tokio) — `crates/server/` + `crates/shared/`
- **Frontend**: Vue 3 + TypeScript (reactive store, no Pinia) — `client/`
- **Desktop**: Tauri 2
- **Voice/Video**: LiveKit (external server)
- **Deploy**: Docker/Podman + Caddy reverse proxy

## Architecture
- **1 instance = 1 serveur** (pas de guilds/servers dans un serveur, modèle TeamSpeak)
- **Multi-serveur côté client** : l'app peut se connecter à plusieurs serveurs simultanément (`Map<serverId, ServerState>`)
- **State sync** : WebSocket avec seq numbers + snapshot (pas de polling REST)
  - Connexion WS → serveur envoie un `Snapshot` complet
  - Events temps réel avec `SequencedEvent { seq, type, data }`
  - Gap detection → auto `RequestSnapshot`
  - Reconnexion avec backoff exponentiel (1s → 30s)

## Key files
| Fichier | Rôle |
|---------|------|
| `crates/shared/src/events.rs` | `ServerEvent`, `ClientEvent`, `SequencedEvent`, `Snapshot` |
| `crates/shared/src/models.rs` | `User`, `Message`, `Channel`, `Role`, `VoiceUserState` |
| `crates/server/src/ws/mod.rs` | WebSocket handler, snapshot, event routing |
| `crates/server/src/state.rs` | `AppState` (broadcast, seq counter, voice state) |
| `crates/server/src/db/` | SQLite queries (users, messages, channels, roles, servers) |
| `crates/server/src/routes/` | REST API (channels, roles, users, invites, livekit, og) |
| `client/src/store.ts` | Store réactif Vue (`reactive()`), state management, WS handling |
| `client/src/api.ts` | REST API client + `createWsConnection()` |
| `client/src/voice.ts` | LiveKit voice/video integration |
| `scripts/install.sh` | One-command deploy (Docker + Caddy + LiveKit) |
| `Dockerfile` | Multi-stage build (Rust + Alpine) |

## Dev commands
```bash
# Dev (backend + frontend)
./scripts/dev.sh
# or manually:
cargo run -p server          # backend on :3000
cd client && bun dev         # frontend with HMR

# Build
cargo build --release -p server
cd client && npm run build

# Type check
cd client && npx vue-tsc --noEmit

# Docker
./scripts/build-image.sh
```

## Environment variables
| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite:./data/data.db` | SQLite path |
| `JWT_SECRET` | required | Token signing key |
| `RUST_LOG` | `server=debug` (dev) / `server=info` (prod) | Log level |
| `SERVER_NAME` | `Sorry Server` | Server display name |
| `LIVEKIT_URL` | required | Public LiveKit WebSocket URL |
| `LIVEKIT_INTERNAL_URL` | `http://localhost:7880` | Internal LiveKit API |
| `LIVEKIT_API_KEY` / `LIVEKIT_API_SECRET` | required | LiveKit auth |
| `UPLOAD_DIR` | `./data/uploads` | File storage path |
| `MAX_FILE_SIZE_MB` | `25` | Upload limit |

## Conventions
- Vue store uses `reactive()` — **NOT Pinia**. All state mutations from async code MUST go through the store proxy (`store.serverStates.get(id)`) not raw object references.
- WebSocket protocol: `serde(tag = "type", content = "data")` for events, `serde(flatten)` for seq wrapper.
- Permissions: bitmask system (25 flags), computed from roles + channel overwrites.
- System roles: ID=1 (Owner), ID=2 (Membre/everyone).
- Frontend comments and variable names can be in French.
