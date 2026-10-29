#!/usr/bin/env bash
# Docker Engine + Compose v2 (`docker compose`) + git, on Ubuntu (x86_64 or arm64).
set -euo pipefail

echo "INSTALLING DEPENDENCIES..."

curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
rm get-docker.sh

# docker without sudo, from your next login on
sudo usermod -aG docker "$USER"

sudo apt-get update
sudo apt-get install -y git

docker --version
sudo docker compose version
git --version
