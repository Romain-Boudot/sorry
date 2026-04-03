#!/usr/bin/env bash
set -e

# Sorry — One-command update
# curl -fsSL https://raw.githubusercontent.com/Romain-Boudot/sorry/main/scripts/update.sh | bash

INSTALL_DIR="${1:-$HOME/sorry}"

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

echo ""
echo -e "${bold}  Sorry — Update${reset}"
echo ""

# Find install dir
if [ -f "./docker-compose.yml" ] && grep -q "sorry" "./docker-compose.yml" 2>/dev/null; then
  INSTALL_DIR="$(pwd)"
elif [ ! -f "$INSTALL_DIR/docker-compose.yml" ]; then
  error "Installation non trouvee dans ./ ni dans $INSTALL_DIR"
fi

cd "$INSTALL_DIR"
ok "Installation: $INSTALL_DIR"

# Detect compose
COMPOSE=""
if command -v podman &>/dev/null; then
  if command -v podman-compose &>/dev/null; then
    COMPOSE="podman-compose"
  elif podman compose version &>/dev/null 2>&1; then
    COMPOSE="podman compose"
  fi
elif command -v docker &>/dev/null; then
  if docker compose version &>/dev/null 2>&1; then
    COMPOSE="docker compose"
  elif command -v docker-compose &>/dev/null; then
    COMPOSE="docker-compose"
  fi
fi

[ -z "$COMPOSE" ] && error "Ni docker ni podman trouve"
ok "Engine: $COMPOSE"

# Check for wget or curl
HTTP_GET=""
if command -v wget &>/dev/null; then
  HTTP_GET="wget"
elif command -v curl &>/dev/null; then
  HTTP_GET="curl"
else
  warn "Ni wget ni curl installe — impossible de verifier la version"
fi

fetch_url() {
  if [ "$HTTP_GET" = "wget" ]; then
    wget -q -O- --timeout=5 "$1" 2>/dev/null
  elif [ "$HTTP_GET" = "curl" ]; then
    curl -sf --max-time 5 "$1" 2>/dev/null
  else
    return 1
  fi
}

check_url() {
  if [ "$HTTP_GET" = "wget" ]; then
    wget -q --spider --timeout=5 "$1" 2>/dev/null
  elif [ "$HTTP_GET" = "curl" ]; then
    curl -sf --max-time 5 -o /dev/null "$1" 2>/dev/null
  else
    return 1
  fi
}

# Get current version
CURRENT=$(fetch_url http://localhost:3000/version || echo "inconnu")
info "Version actuelle: $CURRENT"

# Pull
echo ""
info "Telechargement des nouvelles images..."
$COMPOSE pull

# Restart
info "Redemarrage..."
$COMPOSE up -d

# Wait for health check
info "Verification..."
for i in $(seq 1 15); do
  if check_url http://localhost:3000/health; then
    NEW=$(fetch_url http://localhost:3000/version || echo "inconnu")
    echo ""
    ok "Mis a jour ! $CURRENT -> $NEW"
    echo ""
    echo -e "  ${dim}Logs: cd $INSTALL_DIR && $COMPOSE logs -f${reset}"
    echo ""
    exit 0
  fi
  sleep 2
done

warn "Le serveur met du temps a demarrer, verifie les logs :"
echo -e "  ${dim}cd $INSTALL_DIR && $COMPOSE logs sorry${reset}"
echo ""
