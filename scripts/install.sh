#!/usr/bin/env bash
set -e

# Sorry — One-command deploy

IMAGE="${SORRY_IMAGE:-rg.fr-par.scw.cloud/sorry/sorry:latest}"
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

# ── Helpers ──

gen_secret() { head -c 48 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 32; }

# Read a value from the user, with a default
read_input() {
  local result
  read -r result < /dev/tty
  echo "${result:-$1}"
}

# Validate that a value is a positive integer
validate_int() {
  local val="$1" name="$2"
  if ! [[ "$val" =~ ^[0-9]+$ ]]; then
    error "$name doit etre un nombre entier positif (recu: '$val')"
  fi
}

# ── Check dependencies ──

check_dependencies() {
  local MISSING=""
  for cmd in curl awk grep sort head base64 tr sed; do
    command -v "$cmd" &>/dev/null || MISSING="$MISSING $cmd"
  done
  if [ -n "$MISSING" ]; then
    error "Commandes manquantes:$MISSING — installe-les avant de relancer"
  fi
}

# ── Detect container engine ──

detect_engine() {
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
  ok "Engine: $ENGINE ($COMPOSE)"
}

# ── Deployment mode (local vs public) ──

setup_deployment_mode() {
  echo ""
  echo -e "  ${dim}1)${reset} Local   — acces depuis cette machine uniquement"
  echo -e "  ${dim}2)${reset} Public  — accessible depuis le reseau / internet"
  echo ""
  ask "Mode [1/2]:"
  DEPLOY_MODE=$(read_input "1")

  DOMAIN=""
  USE_HTTPS=false
  HOST="localhost"

  if [ "$DEPLOY_MODE" = "2" ]; then
    PUBLIC_IP=$(curl -s -4 ifconfig.me 2>/dev/null || curl -s -4 icanhazip.com 2>/dev/null || echo "")
    if [ -n "$PUBLIC_IP" ]; then
      ok "IP detectee: $PUBLIC_IP"
    fi
    echo ""

    ask "Nom de domaine (vide = utiliser l'IP):"
    DOMAIN=$(read_input "")

    if [ -n "$DOMAIN" ]; then
      HOST="$DOMAIN"
      ask "Activer HTTPS ? (TLS auto via Caddy) [O/n]:"
      local HTTPS_CHOICE
      HTTPS_CHOICE=$(read_input "o")
      if [[ "$HTTPS_CHOICE" =~ ^[oOyY] ]]; then
        USE_HTTPS=true
      fi
    else
      if [ -z "$PUBLIC_IP" ]; then
        error "Impossible de detecter l'IP publique. Relance avec un nom de domaine."
      fi
      HOST="$PUBLIC_IP"
      info "Les clients se connecteront via http://$HOST"
    fi
  fi
}

# ── Server settings ──

setup_server_info() {
  echo ""
  ask "Nom du serveur [Sorry Server]:"
  SERVER_NAME=$(read_input "Sorry Server")

  ask "Admin username [admin]:"
  ADMIN_USERNAME=$(read_input "admin")

  ask "Admin password (vide = auto-genere):"
  read -rs ADMIN_PASSWORD < /dev/tty
  echo ""
  if [ -z "$ADMIN_PASSWORD" ]; then
    ADMIN_PASSWORD="$(gen_secret)"
    info "Password genere: $ADMIN_PASSWORD"
  fi
}

# ── Voice capacity ──

setup_voice_capacity() {
  echo ""
  echo -e "${bold}Capacite vocale${reset}"
  echo ""
  echo -e "  ${dim}Chaque utilisateur en vocal utilise des ports UDP dedies.${reset}"
  echo -e "  ${dim}Ce n'est pas une limite stricte : au-dela de ce quota, les${reset}"
  echo -e "  ${dim}utilisateurs peuvent toujours se connecter mais la qualite${reset}"
  echo -e "  ${dim}audio/video peut etre degradee (latence plus elevee).${reset}"
  echo ""
  ask "Slots voix simultanes [25]:"
  VOICE_SLOTS=$(read_input "25")
  validate_int "$VOICE_SLOTS" "Slots voix"

  ask "Slots video simultanes [0]:"
  VIDEO_SLOTS=$(read_input "0")
  validate_int "$VIDEO_SLOTS" "Slots video"

  UDP_PORTS=$(( VOICE_SLOTS * 2 + VIDEO_SLOTS * 2 ))
  UDP_START=50000
  UDP_END=$((UDP_START + UDP_PORTS))
  ok "~${VOICE_SLOTS} voix + ~${VIDEO_SLOTS} video — ${UDP_PORTS} ports UDP"
}

# ── Generate secrets ──

generate_secrets() {
  JWT_SECRET="$(gen_secret)"
  LIVEKIT_API_KEY="sorry_$(head -c 8 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 8)"
  LIVEKIT_API_SECRET="$(gen_secret)"

  if [ "$USE_HTTPS" = true ]; then
    LIVEKIT_URL="wss://${HOST}/livekit"
  else
    LIVEKIT_URL="ws://${HOST}/livekit"
  fi
}

# ── Handle existing installation ──

handle_existing_install() {
  if [ -f "$INSTALL_DIR/.env" ]; then
    warn "Installation existante detectee dans $INSTALL_DIR"
    ask "Ecraser la configuration ? Les donnees seront conservees. [o/N]:"
    local OVERWRITE
    OVERWRITE=$(read_input "n")
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
    local EXISTING_JWT EXISTING_LK_KEY EXISTING_LK_SECRET
    EXISTING_JWT=$(grep '^JWT_SECRET=' "$INSTALL_DIR/.env" | cut -d= -f2-)
    EXISTING_LK_KEY=$(grep '^LIVEKIT_API_KEY=' "$INSTALL_DIR/.env" | cut -d= -f2-)
    EXISTING_LK_SECRET=$(grep '^LIVEKIT_API_SECRET=' "$INSTALL_DIR/.env" | cut -d= -f2-)
    [ -n "$EXISTING_JWT" ] && JWT_SECRET="$EXISTING_JWT"
    [ -n "$EXISTING_LK_KEY" ] && LIVEKIT_API_KEY="$EXISTING_LK_KEY"
    [ -n "$EXISTING_LK_SECRET" ] && LIVEKIT_API_SECRET="$EXISTING_LK_SECRET"
  fi
}

# ── Write config files ──

write_configs() {
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
CADDY_HOST=$HOST
PORT=$([ "$USE_HTTPS" = true ] && echo 443 || echo 80)
EOF

  # docker-compose.yml
  cat > docker-compose.yml <<COMPOSE
services:
  sorry:
    image: \${IMAGE:-sorry:latest}
    restart: unless-stopped
    network_mode: host
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
      - LIVEKIT_INTERNAL_URL=http://localhost:7880
      - LIVEKIT_API_KEY=\${LIVEKIT_API_KEY}
      - LIVEKIT_API_SECRET=\${LIVEKIT_API_SECRET}
      - UPLOAD_DIR=\${UPLOAD_DIR:-./data/uploads}
      - MAX_FILE_SIZE_MB=\${MAX_FILE_SIZE_MB:-25}
      - RUST_LOG=\${RUST_LOG:-server=info}
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
    network_mode: host
    environment:
      - CADDY_HOST=\${CADDY_HOST:-localhost}
      - PORT=\${PORT:-80}
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - ./caddy/data:/data
      - ./caddy/config:/config
COMPOSE

  # Caddyfile
  cat > Caddyfile <<'CADDY'
{$CADDY_HOST:localhost}:{$PORT:80} {
    handle /livekit/* {
        uri strip_prefix /livekit
        reverse_proxy localhost:7880
    }
    handle {
        reverse_proxy localhost:3000
    }
}
CADDY

  # livekit.yaml
  local NODE_IP_LINE=""
  if [ -n "$PUBLIC_IP" ]; then
    NODE_IP_LINE="node_ip: $PUBLIC_IP"
  fi
  cat > livekit.yaml <<LK
port: 7880
$NODE_IP_LINE
rtc:
  tcp_port: 7881
  port_range_start: $UDP_START
  port_range_end: $UDP_END
  use_external_ip: true
turn:
  enabled: true
  domain: $HOST
  udp_port: 3478
  tls_port: 0
keys:
  $LIVEKIT_API_KEY: $LIVEKIT_API_SECRET
LK
}

# ── Print summary ──

print_summary() {
  local PROTO="http"
  [ "$USE_HTTPS" = true ] && PROTO="https"

  echo ""
  echo -e "${green}${bold}Sorry is running!${reset}"
  echo ""
  echo -e "  ${bold}URL${reset}        ${PROTO}://$HOST"
  echo ""
  echo -e "  ${bold}Admin${reset}      $ADMIN_USERNAME"
  echo -e "  ${bold}Password${reset}   $ADMIN_PASSWORD"
  echo ""
  echo -e "  ${dim}Config:  $INSTALL_DIR/.env${reset}"
  echo -e "  ${dim}Logs:    cd $INSTALL_DIR && $COMPOSE logs -f${reset}"
  echo -e "  ${dim}Stop:    cd $INSTALL_DIR && $COMPOSE down${reset}"
  echo -e "  ${dim}Update:  curl -fsSL https://raw.githubusercontent.com/Romain-Boudot/sorry/main/scripts/update.sh | bash${reset}"
  echo ""
}

# ══════════════════════════════════════
#  Main
# ══════════════════════════════════════

check_dependencies
detect_engine

echo ""
echo -e "${bold}  Sorry — Self-hosted voice & text${reset}"
echo -e "${dim}  https://github.com/Romain-Boudot/sorry${reset}"
echo ""

echo -e "${bold}Configuration${reset}"

setup_deployment_mode
setup_server_info
setup_voice_capacity
generate_secrets

echo ""
ask "Dossier d'installation [$INSTALL_DIR]:"
INSTALL_DIR=$(read_input "$INSTALL_DIR")

handle_existing_install

info "Installation dans $INSTALL_DIR"
mkdir -p "$INSTALL_DIR/data/uploads"
cd "$INSTALL_DIR"

write_configs

echo ""
info "Recuperation des images..."
$COMPOSE pull

info "Demarrage..."
$COMPOSE up -d --force-recreate

print_summary
