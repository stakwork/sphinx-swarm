docker pull sphinxlightning/sphinx-swarm:latest
docker pull sphinxlightning/sphinx-broker:latest
docker pull sphinxlightning/sphinx-mixer:latest

docker stop swarm.sphinx && docker rm swarm.sphinx

docker-compose -f sphinx.yml up swarm.sphinx -d

docker logs swarm.sphinx --follow
