./scripts/pull_secondbrain.sh

docker stop sphinx-swarm && docker rm sphinx-swarm

docker-compose -f second-brain.yml up sphinx-swarm -d

docker logs sphinx-swarm --follow

./scripts/cleanup.sh
