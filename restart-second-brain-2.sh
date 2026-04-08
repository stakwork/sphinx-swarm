./scripts/pull_secondbrain.sh

docker pull sphinxlightning/sphinx-swarm:latest

docker stop sphinx-swarm && docker rm sphinx-swarm

docker-compose -f second-brain-2.yml up sphinx-swarm -d

docker logs sphinx-swarm --follow