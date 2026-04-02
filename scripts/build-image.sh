#!/usr/bin/env bash
set -e

# Build container image for the current platform
# Compatible with podman and docker
# Usage: ./scripts/build-image.sh [name] [tag] [registry]

IMAGE_NAME="${1:-sorry}"
TAG="${2:-latest}"
REGISTRY="${3:-}"

if [ -n "$REGISTRY" ]; then
  IMAGE="${REGISTRY}/${IMAGE_NAME}:${TAG}"
else
  IMAGE="${IMAGE_NAME}:${TAG}"
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
echo "Building image: $IMAGE"
echo ""

$ENGINE build -t "$IMAGE" -f Dockerfile .

echo ""
echo "Built: $IMAGE"

if [ -n "$REGISTRY" ]; then
  echo "Pushing..."
  $ENGINE push "$IMAGE"
  echo "Pushed: $IMAGE"
else
  echo "To push: $0 $IMAGE_NAME $TAG ghcr.io/youruser"
fi
