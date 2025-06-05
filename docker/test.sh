#!/bin/bash

echo "🚀 Building Docker image (if needed)..."
docker build -t my-anchor-0.31.1 .

echo "Running full build + deploy + test inside Docker..."
docker run -it --rm \
  -v $PWD:/project \
  -v $HOME/.config/solana/id.json:/root/.config/solana/id.json \
  -w /project \
  my-anchor-0.31.1 \
  bash -c "yarn test"


  
