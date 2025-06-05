#!/bin/bash

echo "🚀 Building Anchor program (inside container)..."

export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
export ANCHOR_WALLET=/root/.config/solana/id.json

anchor build
