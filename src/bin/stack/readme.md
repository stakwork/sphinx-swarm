# quick reference

### changing container in swarm

cd sphinx-swarm

sudo vi $HOME/vol/stack/config.yaml (in here you can update the image version)

`./stop.sh jarvis`

`./restart.sh`

### changing swarm container itself

build a new docker image

tag and push

update the image in docker-compose.yaml

pull on the server and `./restart.sh`

# other stuff

### build docker

in src/bin/stack/app `yarn build`

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm .

docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.1.97

docker push sphinxlightning/sphinx-swarm:0.1.97

### run prod stack

in the root of sphinx-swarm directory, create a .env

copy the AWS creds into it, and HOST=xxx

to only start one node:
add --env ONLY_NODE=lnd

docker-compose up -d

docker logs sphinx-swarm --follow

docker-compose down

### update sphinx-swarm itself

docker stop sphinx-swarm && docker rm sphinx-swarm

docker-compose up sphinx-swarm -d

docker logs sphinx-swarm --follow

### remove one volume to reset data

docker volume rm neo4j.sphinx

### update one instance

sudo vi $HOME/vol/stack/config.yaml

update the version

docker pull sphinxlightning/sphinx-jarvis-backend:0.3.2

docker stop jarvis.sphinx && docker rm jarvis.sphinx
docker stop neo4j.sphinx && docker rm neo4j.sphinx

docker-compose up sphinx-swarm -d

docker logs sphinx-swarm --follow

### ps

docker ps -a --format "table {{.Names}}\t{{.Image}}\t{{.RunningFor}}"

### load balancer logs

docker logs load_balancer --follow --since 1m

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

docker logs sphinx-swarm --follow

### once bitcoind is synced

docker stop sphinx-swarm && docker rm sphinx-swarm && docker-compose up sphinx-swarm -d && docker logs sphinx-swarm --follow
