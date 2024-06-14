docker stop swarm.sphinx
docker rm swarm.sphinx
./pull.sh
docker stop $1.sphinx
docker rm $1.sphinx
docker-compose -f sphinxv2.yml up swarm.sphinx -d
docker logs swarm.sphinx --follow