### build docker

in src/bin/stack/app `yarn build`

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm .

docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.1.33

docker push sphinxlightning/sphinx-swarm:0.1.33

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

to only start one node:
add --env ONLY_NODE=lnd

docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up -d

docker logs sphinx-swarm --follow

docker logs load_balancer --follow

docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . down

### remove one volume to reset data

docker volume rm proxy.sphinx

### update one instance

docker pull

docker stop navfiber.sphinx && docker rm navfiber.sphinx

docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . stop sphinx-swarm && docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up --detach --force-recreate sphinx-swarm && docker logs sphinx-swarm --follow

### update sphinx-swarm itself

docker stop sphinx-swarm && docker rm sphinx-swarm && docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up sphinx-swarm -d && docker logs sphinx-swarm --follow

### ps

docker ps --format "table {{.Names}}\t{{.Image}}\t{{.RunningFor}}"
