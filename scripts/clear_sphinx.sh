docker stop mixer.sphinx
docker rm mixer.sphinx

docker stop broker.sphinx
docker rm broker.sphinx

docker stop swarm.sphinx
docker rm swarm.sphinx

docker stop traefik.sphinx
docker rm traefik.sphinx

sudo rm -rf $HOME/vol/stack

docker volume rm $(docker volume ls -q)
