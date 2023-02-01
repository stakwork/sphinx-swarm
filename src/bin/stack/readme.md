### build docker

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm .

docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.1.5

docker push sphinxlightning/sphinx-swarm:0.1.5

### run sphinx swarm in dev

docker network create sphinx-swarm

docker run --name=sphinx-swarm \
 --network=sphinx-swarm \
 --restart=on-failure \
 --volume=/var/run/docker.sock:/var/run/docker.sock \
 --volume=/Users/evanfeenstra/vol:/vol \
 --env DOCKER_RUN=true \
 --publish 8000:8000 \
 --detached \
 sphinx-swarm

### run sphinx swarm in prod

docker run --name=sphinx-swarm \
 --network=sphinx-swarm \
 --restart=on-failure \
 --volume=/var/run/docker.sock:/var/run/docker.sock \
 --volume=/home/admin/vol:/vol \
 --env-file ./.env.prod \
 --publish 8000:8000 \
 --detached \
 sphinxlightning/sphinx-swarm:0.1.5
