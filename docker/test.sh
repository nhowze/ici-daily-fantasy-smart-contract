#!/bin/bash

echo "🚀 Running Anchor tests on Devnet (inside container)..."

export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
export ANCHOR_WALLET=/root/.config/solana/id.json

yarn test
