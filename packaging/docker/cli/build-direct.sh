#!/bin/bash

# Build script for degov-cli Docker image (can be run from CLI directory)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building degov-cli Docker image (direct execution)...${NC}"

# Get the script directory (packaging/docker/cli)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Navigate to project root (3 levels up)
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Image name and tag
IMAGE_NAME="degov-cli"
IMAGE_TAG="${IMAGE_TAG:-latest}"
FDB_VERSION="${FDB_VERSION:-7.3.69}"

echo -e "${YELLOW}Project root: ${PROJECT_ROOT}${NC}"
echo -e "${YELLOW}Building image: ${IMAGE_NAME}:${IMAGE_TAG}${NC}"
echo -e "${YELLOW}FoundationDB version: ${FDB_VERSION}${NC}"

# Build the Docker image from the CLI directory
cd "$SCRIPT_DIR"
docker build \
    --build-arg FDB_VERSION="${FDB_VERSION}" \
    -t "${IMAGE_NAME}:${IMAGE_TAG}" \
    .

echo -e "${GREEN}✓ Build complete!${NC}"
echo -e "${GREEN}Run the CLI with: docker run --rm ${IMAGE_NAME}:${IMAGE_TAG} --help${NC}"
echo -e "${GREEN}Example: docker run --rm ${IMAGE_NAME}:${IMAGE_TAG} dgl --help${NC}"
