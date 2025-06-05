#!/bin/bash

echo "🚀 Building & deploying Anchor program to Devnet..."

export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
export ANCHOR_WALLET=/root/.config/solana/id.json

anchor build && anchor deploy
