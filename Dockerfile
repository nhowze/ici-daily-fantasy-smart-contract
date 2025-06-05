# Use Ubuntu 24.04 — ensures GLIBC >= 2.38 (required for Anchor 0.31.x + Solana 2.0.x)
FROM ubuntu:24.04 as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    bash \
    pkg-config \
    libssl-dev \
    build-essential \
    curl \
    git \
    ca-certificates \
    wget \
    libudev-dev \
    llvm \
    clang \
    lld \
    protobuf-compiler

# Install Rust 1.79
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain 1.79.0

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Solana CLI v2.2.15 from Anza GitHub release
RUN wget https://github.com/anza-xyz/agave/releases/download/v2.2.15/solana-release-x86_64-unknown-linux-gnu.tar.bz2 && \
    tar -xjf solana-release-x86_64-unknown-linux-gnu.tar.bz2 && \
    mv solana-release /usr/local/solana && \
    ln -s /usr/local/solana/bin/solana /usr/local/bin/solana && \
    ln -s /usr/local/solana/bin/solana-cargo-build-sbf /usr/local/bin/solana-cargo-build-sbf && \
    rm solana-release-x86_64-unknown-linux-gnu.tar.bz2

# Verify Solana installation
RUN solana --version

# Install Anchor CLI v0.31.1 from pre-built release (single binary)
RUN wget https://github.com/solana-foundation/anchor/releases/download/v0.31.1/anchor-0.31.1-x86_64-unknown-linux-gnu -O /usr/local/bin/anchor && \
    chmod +x /usr/local/bin/anchor

# Verify Anchor CLI installation
RUN anchor --version

# Verify solana-cargo-build-sbf is installed (now from Solana CLI)
RUN solana-cargo-build-sbf --help || echo "solana-cargo-build-sbf installed (no --version output)"

# Set default workdir
WORKDIR /project
