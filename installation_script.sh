#!/bin/bash

# Exit immediately if a command fails
set -e

# Install dependencies
sudo apt update
sudo apt install -y git curl build-essential

# Install Rust if not already installed
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Clone the repo if it doesn't exist
if [ ! -d "$HOME/amdgpu_top" ]; then
    git clone https://gitlab.freedesktop.org/takayuki.i/amd/amdgpu_top.git ~/amdgpu_top
fi

# Build the project
cd ~/amdgpu_top
cargo build --release

# Install binary to ~/.local/bin
mkdir -p ~/.local/bin
cp target/release/amdgpu_top ~/.local/bin/

echo " amdgpu_top installed!"
echo "Run it with: ~/.local/bin/amdgpu_top"
