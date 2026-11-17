#!/bin/bash
# Pull first (so old layers are replaced, not removed up front), recreate the
# container, then prune dangling layers left by the superseded images.
./scripts/pull_secondbrain.sh

docker stop sphinx-swarm && docker rm sphinx-swarm

docker-compose -f second-brain-2.yml up sphinx-swarm -d

# old image is now untagged and unused: drop dangling layers only
docker image prune -f

docker logs sphinx-swarm --follow
