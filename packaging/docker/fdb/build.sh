#!/bin/bash
set -e

# Build script for the IPv6-fixed FoundationDB Docker image

IMAGE_NAME="${IMAGE_NAME:-degov-fdb}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

echo "Building FoundationDB Docker image with IPv6 fix..."
echo "Image name: $IMAGE_NAME:$IMAGE_TAG"

container build -t "$IMAGE_NAME:$IMAGE_TAG" --arch x86_64 .

echo ""
echo "Build complete!"
echo "To run the image:"
echo "  docker run -e FDB_NETWORKING_MODE=container -e FDB_PORT=4500 $IMAGE_NAME:$IMAGE_TAG"
echo ""
echo "Or with IPv6:"
echo "  docker run --network host -e FDB_NETWORKING_MODE=container -e FDB_PORT=4500 $IMAGE_NAME:$IMAGE_TAG"

