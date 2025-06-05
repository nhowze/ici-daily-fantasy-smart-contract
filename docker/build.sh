#!/bin/bash

echo "🚀 Building Anchor program..."
docker compose run --rm my-anchor bash -c "anchor build"
