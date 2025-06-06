#!/bin/bash

# Force correct RPC on every run
solana config set --url https://damp-tiniest-dew.solana-devnet.quiknode.pro/189722259832d54ae234b019a3a4a8c5cdf9d917

# Show current config (optional — helps debugging)
solana config get

# Start interactive shell
exec /bin/bash
