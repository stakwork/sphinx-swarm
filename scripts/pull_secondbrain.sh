#!/bin/bash
# Pull the images used by the second-brain stack.
# Dangling layers from the replaced tags are pruned at the end (dangling only,
# so images still tagged or in use by containers are never touched).
docker pull sphinxlightning/sphinx-swarm:latest
docker pull sphinxlightning/sphinx-boltwall:latest
# docker pull sphinxlightning/sphinx-jarvis-backend:latest
docker pull ghcr.io/stakwork/stakgraph-mcp:latest
# docker pull ghcr.io/evanfeenstra/chrome-playwright:latest
docker pull ghcr.io/stakwork/stakgraph-standalone:latest

docker image prune -f
