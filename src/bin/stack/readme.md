### build docker

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm .

docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.1.2

docker push sphinxlightning/sphinx-swarm:0.1.2

### run sphinx swarm in dev

docker network create sphinx-swarm

docker run --name=sphinx-swarm \
 --network=sphinx-swarm \
 --restart=on-failure \
 --volume=/var/run/docker.sock:/var/run/docker.sock \
 --volume=/Users/evanfeenstra/vol:/vol \
 -p 8000:8000 \
 -e DOCKER_RUN=true \
 --detached \
 sphinx-swarm

### run sphinx swarm in prod

docker run --name=sphinx-swarm \
 --network=sphinx-swarm \
 --restart=on-failure \
 --volume=/var/run/docker.sock:/var/run/docker.sock \
 --volume=/home/admin/vol:/vol \
 --env-file ./.env.prod \
 -p 8000:8000 \
 --detached \
 sphinxlightning/sphinx-swarm:0.1.2
