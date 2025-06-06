#!/bin/bash

echo "🚀 Building Anchor program (inside container)..."

export ANCHOR_PROVIDER_URL=https://damp-tiniest-dew.solana-devnet.quiknode.pro/189722259832d54ae234b019a3a4a8c5cdf9d917
export ANCHOR_WALLET=/root/.config/solana/id.json

anchor build
