#!/bin/bash

# Force correct RPC on every run
solana config set --url https://api.devnet.solana.com

# Show current config (optional — helps debugging)
solana config get

# Start interactive shell
exec /bin/bash
