#!/bin/bash

echo "🚀 Building Anchor program (inside container)..."

# Set provider + wallet (use your values)
export ANCHOR_PROVIDER_URL=https://damp-tiniest-dew.solana-devnet.quiknode.pro/189722259832d54ae234b019a3a4a8c5cdf9d917
export ANCHOR_WALLET=/root/.config/solana/id.json

# Clean old builds
echo "🧹 Cleaning old build artifacts..."
anchor clean

# Build for local testing (optional, checks syntax)
echo "🔨 Building native Anchor build..."
anchor build

# Build for SBF (for deploy to Devnet/Mainnet)
echo "🎯 Building SBF (solana-cargo-build-sbf)..."
solana-cargo-build-sbf

# Check if .so file was created
if [ -f target/deploy/*.so ]; then
  echo "✅ Build successful! .so file created:"
  ls -lh target/deploy/*.so
else
  echo "❌ Build failed! .so file not found."
  exit 1
fi
