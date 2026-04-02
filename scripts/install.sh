#!/usr/bin/env bash
set -e

# Sorry — One-command deploy

IMAGE="rg.fr-par.scw.cloud/sorry/sorry:latest"
INSTALL_DIR="${INSTALL_DIR:-$HOME/sorry}"

# ── Colors ──
bold="\033[1m"
dim="\033[2m"
green="\033[32m"
cyan="\033[36m"
yellow="\033[33m"
red="\033[31m"
reset="\033[0m"

info()  { echo -e "${cyan}>${reset} $1"; }
ok()    { echo -e "${green}✓${reset} $1"; }
warn()  { echo -e "${yellow}!${reset} $1"; }
error() { echo -e "${red}✗${reset} $1"; exit 1; }
ask()   { echo -en "${bold}$1${reset} "; }

# ── Check dependencies ──
MISSING=""
for cmd in curl awk grep sort head base64 tr sed; do
  command -v "$cmd" &>/dev/null || MISSING="$MISSING $cmd"
done
[ -n "$MISSING" ] && error "Commandes manquantes:$MISSING — installe-les avant de relancer"

# ── Detect container engine ──
ENGINE=""
COMPOSE=""
if command -v podman &>/dev/null; then
  ENGINE="podman"
  if command -v podman-compose &>/dev/null; then
    COMPOSE="podman-compose"
  elif podman compose version &>/dev/null 2>&1; then
    COMPOSE="podman compose"
  fi
elif command -v docker &>/dev/null; then
  ENGINE="docker"
  if docker compose version &>/dev/null 2>&1; then
    COMPOSE="docker compose"
  elif command -v docker-compose &>/dev/null; then
    COMPOSE="docker-compose"
  fi
fi

[ -z "$ENGINE" ] && error "Neither podman nor docker found. Install one first."
[ -z "$COMPOSE" ] && error "$ENGINE found but no compose plugin. Install ${ENGINE}-compose."

echo ""
echo -e "${bold}  Sorry — Self-hosted voice & text${reset}"
echo -e "${dim}  https://github.com/Romain-Boudot/sorry${reset}"
echo ""
ok "Engine: $ENGINE ($COMPOSE)"

# ── Interactive setup ──
gen_secret() { head -c 32 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 32; }

echo ""
echo -e "${bold}Configuration${reset}"
echo ""

# Deploy mode
echo -e "  ${dim}1)${reset} Local   — acces depuis cette machine uniquement"
echo -e "  ${dim}2)${reset} Public  — accessible depuis le reseau / internet"
echo ""
ask "Mode [1/2]:"
read -r DEPLOY_MODE < /dev/tty
DEPLOY_MODE="${DEPLOY_MODE:-1}"

DOMAIN=""
USE_HTTPS=false
HOST="localhost"

if [ "$DEPLOY_MODE" = "2" ]; then
  # Detect public IP
  PUBLIC_IP=$(curl -s -4 ifconfig.me 2>/dev/null || curl -s -4 icanhazip.com 2>/dev/null || echo "")
  if [ -n "$PUBLIC_IP" ]; then
    ok "IP detectee: $PUBLIC_IP"
  fi
  echo ""

  ask "Nom de domaine (vide = utiliser l'IP):"
  read -r DOMAIN < /dev/tty

  if [ -n "$DOMAIN" ]; then
    HOST="$DOMAIN"
    ask "Activer HTTPS ? (TLS auto via Caddy) [O/n]:"
    read -r HTTPS_CHOICE < /dev/tty
    HTTPS_CHOICE="${HTTPS_CHOICE:-o}"
    if [[ "$HTTPS_CHOICE" =~ ^[oOyY] ]]; then
      USE_HTTPS=true
    fi
  else
    HOST="${PUBLIC_IP:-0.0.0.0}"
    info "Les clients se connecteront via http://$HOST"
  fi
fi

echo ""
ask "Nom du serveur [Sorry Server]:"
read -r SERVER_NAME < /dev/tty
SERVER_NAME="${SERVER_NAME:-Sorry Server}"

ask "Admin username [admin]:"
read -r ADMIN_USERNAME < /dev/tty
ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"

ask "Admin password (vide = auto-genere):"
read -rs ADMIN_PASSWORD < /dev/tty
echo ""
if [ -z "$ADMIN_PASSWORD" ]; then
  ADMIN_PASSWORD="$(gen_secret)"
  info "Password genere: $ADMIN_PASSWORD"
fi

# Voice capacity
echo ""
echo -e "${bold}Capacite vocale${reset}"
echo ""
echo -e "  ${dim}Chaque utilisateur en vocal utilise des ports UDP dedies.${reset}"
echo -e "  ${dim}Ce n'est pas une limite stricte : au-dela de ce quota, les${reset}"
echo -e "  ${dim}utilisateurs peuvent toujours se connecter mais la qualite${reset}"
echo -e "  ${dim}audio/video peut etre degradee (latence plus elevee).${reset}"
echo ""
ask "Slots voix simultanes [25]:"
read -r VOICE_SLOTS < /dev/tty
VOICE_SLOTS="${VOICE_SLOTS:-25}"

ask "Slots video simultanes [0]:"
read -r VIDEO_SLOTS < /dev/tty
VIDEO_SLOTS="${VIDEO_SLOTS:-0}"

UDP_PORTS=$(( VOICE_SLOTS * 2 + VIDEO_SLOTS * 2 ))
if [ "$UDP_PORTS" -gt 200 ]; then
  USE_HOST_NETWORK=true
  ok "~${VOICE_SLOTS} voix + ~${VIDEO_SLOTS} video — mode reseau direct"
else
  USE_HOST_NETWORK=false
  UDP_START=50000
  UDP_END=$((UDP_START + UDP_PORTS))
  ok "~${VOICE_SLOTS} voix + ~${VIDEO_SLOTS} video — ${UDP_PORTS} ports UDP"
fi

# Generate secrets
JWT_SECRET="$(gen_secret)"
LIVEKIT_API_KEY="sorry_$(head -c 8 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 8)"
LIVEKIT_API_SECRET="$(gen_secret)"

# Derive LiveKit URL
if [ "$USE_HTTPS" = true ]; then
  LIVEKIT_URL="wss://${HOST}/livekit"
else
  LIVEKIT_URL="ws://${HOST}/livekit"
fi

# ── Install ──
echo ""
ask "Dossier d'installation [$INSTALL_DIR]:"
read -r CUSTOM_DIR < /dev/tty
INSTALL_DIR="${CUSTOM_DIR:-$INSTALL_DIR}"

# Check if already installed
if [ -f "$INSTALL_DIR/.env" ]; then
  warn "Installation existante detectee dans $INSTALL_DIR"
  ask "Ecraser la configuration ? Les donnees seront conservees. [o/N]:"
  read -r OVERWRITE < /dev/tty
  OVERWRITE="${OVERWRITE:-n}"
  if [[ ! "$OVERWRITE" =~ ^[oOyY] ]]; then
    info "Mise a jour uniquement (pull + restart)..."
    cd "$INSTALL_DIR"
    $COMPOSE pull
    $COMPOSE up -d
    ok "Mis a jour !"
    exit 0
  fi
  # Keep existing secrets
  info "Conservation des secrets existants..."
  EXISTING_JWT=$(grep '^JWT_SECRET=' "$INSTALL_DIR/.env" | cut -d= -f2-)
  EXISTING_LK_KEY=$(grep '^LIVEKIT_API_KEY=' "$INSTALL_DIR/.env" | cut -d= -f2-)
  EXISTING_LK_SECRET=$(grep '^LIVEKIT_API_SECRET=' "$INSTALL_DIR/.env" | cut -d= -f2-)
  [ -n "$EXISTING_JWT" ] && JWT_SECRET="$EXISTING_JWT"
  [ -n "$EXISTING_LK_KEY" ] && LIVEKIT_API_KEY="$EXISTING_LK_KEY"
  [ -n "$EXISTING_LK_SECRET" ] && LIVEKIT_API_SECRET="$EXISTING_LK_SECRET"
fi

info "Installation dans $INSTALL_DIR"
mkdir -p "$INSTALL_DIR/data/uploads" "$INSTALL_DIR/caddy"
cd "$INSTALL_DIR"

# .env
cat > .env <<EOF
IMAGE=$IMAGE
SERVER_NAME=$SERVER_NAME
ADMIN_USERNAME=$ADMIN_USERNAME
ADMIN_PASSWORD=$ADMIN_PASSWORD
JWT_SECRET=$JWT_SECRET
LIVEKIT_URL=$LIVEKIT_URL
LIVEKIT_API_KEY=$LIVEKIT_API_KEY
LIVEKIT_API_SECRET=$LIVEKIT_API_SECRET
EOF

if [ "$USE_HTTPS" = true ]; then
  cat >> .env <<EOF
CADDY_HOST=$HOST
PORT=443
EOF
else
  cat >> .env <<EOF
CADDY_HOST=$HOST
PORT=80
EOF
fi

# docker-compose.yml
if [ "$USE_HOST_NETWORK" = true ]; then
  # Host network mode — LiveKit binds directly, no port mapping
  cat > docker-compose.yml <<'COMPOSE'
services:
  sorry:
    image: ${IMAGE:-sorry:latest}
    restart: unless-stopped
    extra_hosts:
      - "host.docker.internal:host-gateway"
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    environment:
      - DATABASE_URL=sqlite:./data/data.db
      - JWT_SECRET=${JWT_SECRET}
      - SERVER_NAME=${SERVER_NAME:-Sorry Server}
      - ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-}
      - LIVEKIT_URL=${LIVEKIT_URL}
      - LIVEKIT_INTERNAL_URL=http://host.docker.internal:7880
      - LIVEKIT_API_KEY=${LIVEKIT_API_KEY}
      - LIVEKIT_API_SECRET=${LIVEKIT_API_SECRET}
      - UPLOAD_DIR=./data/uploads
    volumes:
      - ./data:/app/data

  livekit:
    image: livekit/livekit-server:latest
    restart: unless-stopped
    network_mode: host
    command: --config /etc/livekit.yaml
    volumes:
      - ./livekit.yaml:/etc/livekit.yaml

  caddy:
    image: caddy:2
    restart: unless-stopped
    extra_hosts:
      - "livekit:host-gateway"
    ports:
      - "80:80"
      - "443:443"
    environment:
      - CADDY_HOST=${CADDY_HOST:-localhost}
      - PORT=${PORT:-80}
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - ./caddy:/data
COMPOSE
else
  # Port mapping mode — LiveKit ports mapped by Docker
  cat > docker-compose.yml <<COMPOSE
services:
  sorry:
    image: \${IMAGE:-sorry:latest}
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    environment:
      - DATABASE_URL=sqlite:./data/data.db
      - JWT_SECRET=\${JWT_SECRET}
      - SERVER_NAME=\${SERVER_NAME:-Sorry Server}
      - ADMIN_USERNAME=\${ADMIN_USERNAME:-admin}
      - ADMIN_PASSWORD=\${ADMIN_PASSWORD:-}
      - LIVEKIT_URL=\${LIVEKIT_URL}
      - LIVEKIT_INTERNAL_URL=http://livekit:7880
      - LIVEKIT_API_KEY=\${LIVEKIT_API_KEY}
      - LIVEKIT_API_SECRET=\${LIVEKIT_API_SECRET}
      - UPLOAD_DIR=./data/uploads
    volumes:
      - ./data:/app/data

  livekit:
    image: livekit/livekit-server:latest
    restart: unless-stopped
    command: --config /etc/livekit.yaml
    ports:
      - "7881:7881"
      - "${UDP_START}-${UDP_END}:${UDP_START}-${UDP_END}/udp"
    volumes:
      - ./livekit.yaml:/etc/livekit.yaml

  caddy:
    image: caddy:2
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    environment:
      - CADDY_HOST=\${CADDY_HOST:-localhost}
      - PORT=\${PORT:-80}
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - ./caddy:/data
COMPOSE
fi

# Caddyfile
cat > Caddyfile <<'CADDY'
{$CADDY_HOST:localhost}:{$PORT:80} {
    handle /livekit/* {
        uri strip_prefix /livekit
        reverse_proxy livekit:7880
    }
    handle {
        reverse_proxy sorry:3000
    }
}
CADDY

# LiveKit config
if [ "$USE_HOST_NETWORK" = true ]; then
  LK_PORT_START=50000
  LK_PORT_END=60000
else
  LK_PORT_START=$UDP_START
  LK_PORT_END=$UDP_END
fi

cat > livekit.yaml <<LK
port: 7880
rtc:
  tcp_port: 7881
  port_range_start: $LK_PORT_START
  port_range_end: $LK_PORT_END
  use_external_ip: true
keys:
  \${LIVEKIT_API_KEY}: \${LIVEKIT_API_SECRET}
LK

# ── Start ──
echo ""
info "Demarrage..."
$COMPOSE up -d

echo ""
echo -e "${green}${bold}Sorry is running!${reset}"
echo ""
if [ "$USE_HTTPS" = true ]; then
  echo -e "  ${bold}URL${reset}        https://$HOST"
else
  echo -e "  ${bold}URL${reset}        http://$HOST"
fi
echo ""
echo -e "  ${bold}Admin${reset}      $ADMIN_USERNAME"
echo -e "  ${bold}Password${reset}   $ADMIN_PASSWORD"
echo ""
echo -e "  ${dim}Config:  $INSTALL_DIR/.env${reset}"
echo -e "  ${dim}Logs:    cd $INSTALL_DIR && $COMPOSE logs -f${reset}"
echo -e "  ${dim}Stop:    cd $INSTALL_DIR && $COMPOSE down${reset}"
echo -e "  ${dim}Update:  cd $INSTALL_DIR && $COMPOSE pull && $COMPOSE up -d${reset}"
echo ""
