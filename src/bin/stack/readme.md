### build docker

in src/bin/stack/app `yarn build`

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm .

docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.1.36

docker push sphinxlightning/sphinx-swarm:0.1.36

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

sudo vi $HOME/vol/stack/config.json

update the version

docker pull image:version

docker stop jarvis.sphinx && docker rm jarvis.sphinx

docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . stop sphinx-swarm && docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up --detach sphinx-swarm && docker logs sphinx-swarm --follow

### update sphinx-swarm itself

docker stop sphinx-swarm && docker rm sphinx-swarm && docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up sphinx-swarm -d && docker logs sphinx-swarm --follow

### ps

docker ps --format "table {{.Names}}\t{{.Image}}\t{{.RunningFor}}"

# deps

install docker, docker-compose, and git on a new EC2:

### docker

curl -fsSL https://get.docker.com/ -o get-docker.sh

sh get-docker.sh

sudo usermod -aG docker $USER

### docker compose latest version

sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) -o /usr/local/bin/docker-compose

sudo chmod +x /usr/local/bin/docker-compose

docker-compose version

### git

sudo apt update

sudo apt install git

### clone the repo

git clone https://github.com/stakwork/sphinx-swarm.git

### aws:

create an A record like `*.swarmx.sphinx.chat` to the IP of the instance

### setup first time (only bitcoin):

export ONLY_NODE=bitcoind
export HOST=swarm5.sphinx.chat

copy the envs from .env.md

docker network create sphinx-swarm

docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up -d

### once bitcoind is synced

export HOST=swarm5.sphinx.chat

copy the envs from .env.md

docker stop sphinx-swarm && docker rm sphinx-swarm && docker-compose -f ./src/bin/stack/stack-prod.yml --project-directory . up sphinx-swarm -d && docker logs sphinx-swarm --follow
