### build docker

in src/bin/stack/app `yarn build`

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm .

docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.1.42

docker push sphinxlightning/sphinx-swarm:0.1.42

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

in the root of sphinx-swarm directory, create a .env

copy the AWS creds into it, and HOST=xxx

to only start one node:
add --env ONLY_NODE=lnd

docker-compose up -d

docker logs sphinx-swarm --follow

docker logs load_balancer --follow

docker-compose down

### remove one volume to reset data

docker volume rm proxy.sphinx

### update one instance

sudo vi $HOME/vol/stack/config.json

update the version

docker pull sphinxlightning/sphinx-proxy:0.1.18

docker stop relay.sphinx && docker rm relay.sphinx

docker-compose stop sphinx-swarm && docker-compose up --detach sphinx-swarm && docker logs sphinx-swarm --follow

### update sphinx-swarm itself

docker stop sphinx-swarm && docker rm sphinx-swarm && docker-compose up sphinx-swarm -d && docker logs sphinx-swarm --follow

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

in the root of sphinx-swarm directory, create a .env

copy the AWS creds into it, and HOST=xxx

export ONLY_NODE=bitcoind

docker network create sphinx-swarm

docker-compose up -d

### once bitcoind is synced

in the root of sphinx-swarm directory, create a .env

copy the AWS creds into it, and HOST=xxx

docker stop sphinx-swarm && docker rm sphinx-swarm && docker-compose up sphinx-swarm -d && docker logs sphinx-swarm --follow
