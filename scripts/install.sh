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

# Generate secrets
JWT_SECRET="$(gen_secret)"
LIVEKIT_API_KEY="sorry_$(head -c 8 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 8)"
LIVEKIT_API_SECRET="$(gen_secret)"
S3_ACCESS_KEY="sorry_s3"
S3_SECRET_KEY="$(gen_secret)"

# Find the largest free UDP port range (between 49152-65535)
find_largest_free_range() {
  local used_system="" used_containers=""

  # System ports (listening + established)
  if command -v ss &>/dev/null; then
    used_system=$(ss -aunH 2>/dev/null | awk '{print $4}' | grep -oE '[0-9]+$')
  elif command -v netstat &>/dev/null; then
    used_system=$(netstat -an 2>/dev/null | grep udp | awk '{print $4}' | grep -oE '[0-9]+$')
  fi

  # Avoid kernel ephemeral port range entirely
  local eph_start=32768 eph_end=60999
  if [ -f /proc/sys/net/ipv4/ip_local_port_range ]; then
    read -r eph_start eph_end < /proc/sys/net/ipv4/ip_local_port_range
  fi

  # Container-allocated ports (docker/podman)
  if command -v "$ENGINE" &>/dev/null; then
    used_containers=$($ENGINE ps --format '{{.Ports}}' 2>/dev/null | grep -oE '[0-9]+-[0-9]+' | while read -r range; do
      start="${range%-*}"; end="${range#*-}"
      seq "$start" "$end"
    done)
    used_containers="$used_containers
$($ENGINE ps --format '{{.Ports}}' 2>/dev/null | grep -oE ':[0-9]+->|:[0-9]+/' | grep -oE '[0-9]+')"
  fi

  local used
  used=$(printf '%s\n%s' "$used_system" "$used_containers" | grep -E '^[0-9]+$' | sort -nu)

  # Search in two safe zones: before and after the ephemeral range
  # Zone 1: 10000 - eph_start
  # Zone 2: eph_end+1 - 65535
  local best_start=10000 best_size=0

  for zone_start in 10000 $((eph_end + 1)); do
    if [ "$zone_start" -eq 10000 ]; then
      zone_end=$((eph_start - 1))
    else
      zone_end=65535
    fi
    [ "$zone_start" -ge "$zone_end" ] && continue

    local cursor=$zone_start
    for port in $used; do
      [ "$port" -lt "$cursor" ] && continue
      [ "$port" -gt "$zone_end" ] && break
      local gap=$((port - cursor))
      if [ "$gap" -gt "$best_size" ]; then
        best_start=$cursor
        best_size=$gap
      fi
      cursor=$((port + 1))
    done

    local gap=$((zone_end + 1 - cursor))
    if [ "$gap" -gt "$best_size" ]; then
      best_start=$cursor
      best_size=$gap
    fi
  done

  echo "$best_start $best_size"
}

read -r UDP_START UDP_SIZE <<< "$(find_largest_free_range)"
# Cap at 10000 ports max (more than enough)
[ "$UDP_SIZE" -gt 10000 ] && UDP_SIZE=10000
UDP_END=$((UDP_START + UDP_SIZE - 1))
UDP_VOICE=$((UDP_SIZE / 2))
UDP_VIDEO=$((UDP_SIZE / 4))
ok "Ports UDP: ${UDP_START}-${UDP_END} (${UDP_SIZE} ports, ~${UDP_VOICE} voix | ~${UDP_VIDEO} voix+video)"

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

info "Installation dans $INSTALL_DIR"
mkdir -p "$INSTALL_DIR/data" "$INSTALL_DIR/minio" "$INSTALL_DIR/caddy"
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
S3_ACCESS_KEY=$S3_ACCESS_KEY
S3_SECRET_KEY=$S3_SECRET_KEY
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
cat > docker-compose.yml <<'COMPOSE'
services:
  sorry:
    image: ${IMAGE:-sorry:latest}
    restart: unless-stopped
    environment:
      - DATABASE_URL=sqlite:./data/data.db
      - JWT_SECRET=${JWT_SECRET}
      - SERVER_NAME=${SERVER_NAME:-Sorry Server}
      - ADMIN_USERNAME=${ADMIN_USERNAME:-admin}
      - ADMIN_PASSWORD=${ADMIN_PASSWORD:-}
      - LIVEKIT_URL=${LIVEKIT_URL}
      - LIVEKIT_INTERNAL_URL=http://livekit:7880
      - LIVEKIT_API_KEY=${LIVEKIT_API_KEY}
      - LIVEKIT_API_SECRET=${LIVEKIT_API_SECRET}
      - S3_ENDPOINT=http://minio:9000
      - S3_BUCKET=${S3_BUCKET:-uploads}
      - S3_ACCESS_KEY=${S3_ACCESS_KEY}
      - S3_SECRET_KEY=${S3_SECRET_KEY}
    volumes:
      - ./data:/app/data
    depends_on:
      - minio

  minio:
    image: minio/minio:latest
    restart: unless-stopped
    command: server /data --console-address ":9001"
    environment:
      - MINIO_ROOT_USER=${S3_ACCESS_KEY:-minioadmin}
      - MINIO_ROOT_PASSWORD=${S3_SECRET_KEY:-minioadmin}
    volumes:
      - ./minio:/data

  livekit:
    image: livekit/livekit-server:latest
    restart: unless-stopped
    command: --config /etc/livekit.yaml
    ports:
      - "7881:7881"
      - "UDPRANGE"
    volumes:
      - ./livekit.yaml:/etc/livekit.yaml

  caddy:
    image: caddy:2
    restart: unless-stopped
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
cat > livekit.yaml <<'LK'
port: 7880
rtc:
  tcp_port: 7881
  port_range_start: UDPSTART
  port_range_end: UDPEND
  use_external_ip: true
keys:
  ${LIVEKIT_API_KEY}: ${LIVEKIT_API_SECRET}
LK

# Inject dynamic port range (sed -i works differently on macOS vs Linux)
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' "s|UDPRANGE|${UDP_START}-${UDP_END}:${UDP_START}-${UDP_END}/udp|g" docker-compose.yml
  sed -i '' "s|UDPSTART|${UDP_START}|g" livekit.yaml
  sed -i '' "s|UDPEND|${UDP_END}|g" livekit.yaml
else
  sed -i "s|UDPRANGE|${UDP_START}-${UDP_END}:${UDP_START}-${UDP_END}/udp|g" docker-compose.yml
  sed -i "s|UDPSTART|${UDP_START}|g" livekit.yaml
  sed -i "s|UDPEND|${UDP_END}|g" livekit.yaml
fi

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
echo -e "  ${bold}UDP${reset}        ${UDP_START}-${UDP_END} (~${UDP_VOICE} voix | ~${UDP_VIDEO} voix+video)"
echo ""
echo -e "  ${dim}Config:  $INSTALL_DIR/.env${reset}"
echo -e "  ${dim}Logs:    cd $INSTALL_DIR && $COMPOSE logs -f${reset}"
echo -e "  ${dim}Stop:    cd $INSTALL_DIR && $COMPOSE down${reset}"
echo -e "  ${dim}Update:  cd $INSTALL_DIR && $COMPOSE pull && $COMPOSE up -d${reset}"
echo ""
