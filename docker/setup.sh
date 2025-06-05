#!/bin/bash

echo "🚀 Making docker scripts executable..."
chmod +x docker/entrypoint.sh
chmod +x docker/build.sh
chmod +x docker/run.sh
chmod +x docker/clean.sh
chmod +x docker/test.sh

echo "✅ Done — all scripts now executable."
