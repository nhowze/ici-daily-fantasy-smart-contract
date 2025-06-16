#!/bin/bash

echo "🧹 Removing old IDL files..."
rm -f ~/dfs_frontend/dfs_rust_crons/config/idl/fantasy_sports.json
rm -f ~/dfs_frontend/dfs_reactJS/src/config/idl/fantasy_sports.json
rm -f ~/dfs_frontend/dfs_middleware/config/idl/fantasy_sports.json

echo "📦 Copying IDL to Rust crons..."
cp target/idl/fantasy_sports.json ~/dfs_frontend/dfs_rust_crons/config/idl/

echo "🌐 Copying IDL to React frontend..."
cp target/idl/fantasy_sports.json ~/dfs_frontend/dfs_reactJS/src/config/idl/

echo "🌐 Copying IDL to React frontend..."
cp target/idl/fantasy_sports.json ~/dfs_frontend/dfs_middleware/config/idl/

echo "✅ IDL copied and old versions removed."
