#!/usr/bin/env bash
set -e

# Build container image for the current platform
# Compatible with podman and docker
# Usage: ./scripts/build-image.sh [name] [tag] [registry]
# If tag is omitted, uses version from Cargo.toml

IMAGE_NAME="${1:-sorry}"
REGISTRY="${3:-}"

# Read version from Cargo.toml
VERSION=$(grep '^version' crates/server/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
TAG="${2:-$VERSION}"

if [ -n "$REGISTRY" ]; then
  IMAGE="${REGISTRY}/${IMAGE_NAME}:${TAG}"
  IMAGE_LATEST="${REGISTRY}/${IMAGE_NAME}:latest"
else
  IMAGE="${IMAGE_NAME}:${TAG}"
  IMAGE_LATEST="${IMAGE_NAME}:latest"
fi

# Detect container engine
if command -v podman &>/dev/null; then
  ENGINE="podman"
elif command -v docker &>/dev/null; then
  ENGINE="docker"
else
  echo "Error: neither podman nor docker found"
  exit 1
fi

echo "Using engine: $ENGINE"
echo "Version: $VERSION"
echo "Building image: $IMAGE"
echo ""

$ENGINE build -t "$IMAGE" -t "$IMAGE_LATEST" -f Dockerfile .

echo ""
echo "Built: $IMAGE"
echo "Tagged: $IMAGE_LATEST"

if [ -n "$REGISTRY" ]; then
  echo "Pushing..."
  $ENGINE push "$IMAGE"
  $ENGINE push "$IMAGE_LATEST"
  echo "Pushed: $IMAGE + latest"
else
  echo "To push: $0 $IMAGE_NAME $TAG registry.example.com/user"
fi
