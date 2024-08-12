docker pull sphinxlightning/sphinx-swarm:latest
docker pull sphinxlightning/sphinx-broker:latest
docker pull sphinxlightning/sphinx-mixer:latest
docker pull sphinxlightning/sphinx-tribes-v2:latest
docker pull sphinxlightning/cln-sphinx:latest
docker pull sphinxlightning/sphinx-bot:latest
docker pull sphinxlightning/sphinx-builtin-bots:latest

docker stop swarm.sphinx && docker rm swarm.sphinx

docker-compose -f sphinxv2.yml up swarm.sphinx -d

docker logs swarm.sphinx --follow
