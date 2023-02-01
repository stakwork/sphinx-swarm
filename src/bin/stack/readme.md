### build docker

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm .

docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.1.17

docker push sphinxlightning/sphinx-swarm:0.1.17

### run sphinx swarm in dev

docker network create sphinx-swarm

docker run --name=sphinx-swarm \
 --name swarm \
 --network=sphinx-swarm \
 --restart=on-failure \
 --volume=/var/run/docker.sock:/var/run/docker.sock \
 --volume=/Users/evanfeenstra/vol:/vol \
 --env DOCKER_RUN=true \
 --publish 8000:8000 \
 --detach \
 sphinx-swarm

### run prod stack

copy the .env.md

docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up -d

docker logs sphinx-swarm --follow

docker logs load_balancer --follow

docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . down

### run sphinx swarm in prod

docker pull traefik:v2.2.1

to only start one node:
add --env ONLY_NODE=lnd

docker run --name=sphinx-swarm \
 --name swarm \
 --network=sphinx-swarm \
 --restart=on-failure \
 --volume=/var/run/docker.sock:/var/run/docker.sock \
 --volume=/home/admin/vol:/vol \
 --env-file ./.env.prod \
 --env TRAEFIK_INSECURE=true \
 --publish 8000:8000 \
 --detach \
 sphinxlightning/sphinx-swarm:0.1.16

docker stop swarm && docker rm swarm

docker stop load_balancer.sphinx && docker rm load_balancer.sphinx

docker logs swarm --follow

docker logs load_balancer.sphinx --follow
