#!/bin/bash

echo "🚀 Cleaning project and program builds..."

# Project-level cleanup
echo "🧹 Removing project Cargo.lock and target/ ..."
rm -f Cargo.lock
rm -rf target

# Program-level cleanup
echo "🧹 Removing program Cargo.lock and target/ ..."
rm -f programs/fantasy_sports/Cargo.lock
rm -rf programs/fantasy_sports/target

# Optional — clean Anchor artifacts
echo "🧹 Removing .anchor cache ..."
rm -rf .anchor

echo "✅ Clean complete — ready to rebuild!"
