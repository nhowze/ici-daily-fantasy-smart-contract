#!/bin/bash

echo "🚀 Running Anchor tests (skip build + skip deploy) — using tests/**/*.ts ..."
echo "🔗 Using workspace: anchor.workspace.FantasySports will work ✅"
echo "🌐 Target cluster: DEVNET (https://api.devnet.solana.com)"

# Set test environment
export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
export ANCHOR_WALLET=/root/.config/solana/id.json

# Run Anchor tests
anchor test --skip-build --skip-deploy
