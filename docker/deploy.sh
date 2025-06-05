#!/bin/bash

echo "🚀 Building & deploying Anchor program to Devnet..."
docker compose run --rm my-anchor bash -c "
export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
export ANCHOR_WALLET=/root/.config/solana/id.json
anchor build && anchor deploy
"
