#!/bin/bash

echo "🚀 Running Anchor tests (skip build + skip deploy) — using tests/**/*.ts ..."
echo "🔗 Using workspace: anchor.workspace.FantasySports will work ✅"

# Optional — Clean yarn cache if you want (safe)
# echo "🧹 Cleaning yarn cache and reinstalling dependencies..."
# yarn cache clean
# rm -rf node_modules yarn.lock
# yarn install

# Set test environment
export ANCHOR_PROVIDER_URL=https://damp-tiniest-dew.solana-devnet.quiknode.pro/189722259832d54ae234b019a3a4a8c5cdf9d917
export ANCHOR_WALLET=/root/.config/solana/id.json

# Run Anchor tests
anchor test --skip-build --skip-deploy
