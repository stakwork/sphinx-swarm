echo "pull images"
docker pull sphinxlightning/sphinx-broker:latest
docker pull sphinxlightning/sphinx-mixer:latest
docker pull sphinxlightning/sphinx-swarm:latest
docker pull sphinxlightning/sphinx-tribes-v2:latest

echo "stop swarm.sphinx"
docker stop swarm.sphinx
docker rm swarm.sphinx

echo "stop $1.sphinx"
docker stop $1.sphinx
docker rm $1.sphinx

echo "start sphinxv2"
docker-compose -f sphinxv2.yml up swarm.sphinx -d
docker logs swarm.sphinx --follow