#!/bin/bash

# Rebuild image with no-cache using docker-compose
echo "🚀 Building Docker Compose image with --no-cache..."
docker-compose build --no-cache

# Run container with interactive shell
echo "🚀 Running Docker Compose container..."
docker-compose run --rm my-anchor bash
