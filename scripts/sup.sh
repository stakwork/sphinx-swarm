#!/bin/bash
# GENERATED FILE - do not edit by hand.
# Regenerate with:  cargo run --bin sup
# Source of truth:  src/secondbrain.rs (services) and src/images/*.rs (image repos)
#
# Usage:  ./scripts/sup.sh <service>
#
# Pulls the service's image, recreates the swarm container, removes the
# service's container so the swarm brings it back up on the new image,
# then follows the service's logs.

cd "$(dirname "$0")/.." || exit 1

case "$1" in
  bifrost)
    IMAGE="ghcr.io/stakwork/stakgraph-gateway:latest"
    CONTAINER="bifrost.sphinx"
    ;;
  boltwall)
    IMAGE="sphinxlightning/sphinx-boltwall:latest"
    CONTAINER="boltwall.sphinx"
    ;;
  bot)
    IMAGE="sphinxlightning/sphinx-bot:latest"
    CONTAINER="bot.sphinx"
    ;;
  browser)
    IMAGE="ghcr.io/stakwork/strut-browser:latest"
    CONTAINER="browser.sphinx"
    ;;
  graphmindset)
    IMAGE="sphinxlightning/graphmindset:latest"
    CONTAINER="graphmindset.sphinx"
    ;;
  hermes)
    IMAGE="nousresearch/hermes-agent:latest"
    CONTAINER="hermes.sphinx"
    ;;
  hive-relay)
    IMAGE="sphinxlightning/hive-relay:latest"
    CONTAINER="hive-relay.sphinx"
    ;;
  jarvis)
    IMAGE="sphinxlightning/sphinx-jarvis-backend:latest"
    CONTAINER="jarvis.sphinx"
    ;;
  llama)
    IMAGE="ghcr.io/ggerganov/llama.cpp:server-cuda"
    CONTAINER="llama.sphinx"
    ;;
  navfiber)
    IMAGE="sphinxlightning/sphinx-nav-fiber:latest"
    CONTAINER="navfiber.sphinx"
    ;;
  neo4j)
    IMAGE="neo4j:5.19.0"
    CONTAINER="neo4j.sphinx"
    ;;
  quickwit)
    IMAGE="quickwit/quickwit:latest"
    CONTAINER="quickwit.sphinx"
    ;;
  redis)
    IMAGE="redis:latest"
    CONTAINER="redis.sphinx"
    ;;
  repo2graph)
    IMAGE="ghcr.io/stakwork/stakgraph-mcp:latest"
    CONTAINER="repo2graph.sphinx"
    ;;
  stakgraph)
    IMAGE="ghcr.io/stakwork/stakgraph-standalone:latest"
    CONTAINER="stakgraph.sphinx"
    ;;
  vector)
    IMAGE="timberio/vector:latest-distroless-libc"
    CONTAINER="vector.sphinx"
    ;;
  *)
    echo "=> invalid service name! '$1'"
    echo "=> valid services: bifrost boltwall bot browser graphmindset hermes hive-relay jarvis llama navfiber neo4j quickwit redis repo2graph stakgraph vector"
    exit 1
    ;;
esac

echo "=> pull $IMAGE"
docker pull "$IMAGE" || exit 1

echo "=> stop sphinx-swarm"
docker stop sphinx-swarm && docker rm sphinx-swarm

echo "=> stop $CONTAINER"
docker stop "$CONTAINER" && docker rm "$CONTAINER"

echo "=> start sphinx-swarm"
docker-compose -f second-brain-2.yml up sphinx-swarm -d

# the swarm recreates the container on startup; wait for it before tailing
echo "=> waiting for $CONTAINER"
for _ in $(seq 1 120); do
  docker inspect "$CONTAINER" >/dev/null 2>&1 && break
  sleep 1
done

docker logs "$CONTAINER" --follow
