#!/bin/bash

echo "🐳 Building Docker image with COMPOSE_BAKE..."
COMPOSE_BAKE=true BUILDKIT_PROGRESS=plain docker compose build
