# quick reference

### changing container in swarm

`cd sphinx-swarm`

`sudo vi $HOME/vol/stack/config.yaml` (in here you can update the image version)

`./stop.sh jarvis`

`./restart.sh`

### changing swarm container itself

build a new docker image

tag and push

update the image in docker-compose.yaml

pull on the server and `./restart.sh`

# other stuff

### run prod stack

in the root of sphinx-swarm directory, create a `.env`

copy the AWS creds into it, and `HOST=xxx`

to only start one node:
add `--env ONLY_NODE=lnd`

`docker-compose up -d`

`docker logs sphinx-swarm --follow`

`docker-compose down`

### update sphinx-swarm itself

`docker stop sphinx-swarm && docker rm sphinx-swarm`

`docker-compose up sphinx-swarm -d`

`docker logs sphinx-swarm --follow`

### remove one volume to reset data

`docker volume rm neo4j.sphinx`
`docker volume rm elastic.sphinx`

### update one instance

`sudo vi $HOME/vol/stack/config.yaml`

update the version

`docker pull sphinxlightning/sphinx-jarvis-backend:0.3.2`

`docker stop jarvis.sphinx && docker rm jarvis.sphinx`
`docker stop neo4j.sphinx && docker rm neo4j.sphinx`
`docker stop elastic.sphinx && docker rm elastic.sphinx`

`docker-compose up sphinx-swarm -d`

`docker logs sphinx-swarm --follow`

### ps

`docker ps -a --format "table {{.Names}}\t{{.Image}}\t{{.RunningFor}}"`

### load balancer logs

`docker logs load_balancer --follow --since 1m`

# deps

install docker, docker-compose, and git on a new EC2:

### docker

`curl -fsSL https://get.docker.com/ -o get-docker.sh`

`sh get-docker.sh`

`sudo usermod -aG docker $USER`

### docker compose latest version

`sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) -o /usr/local/bin/docker-compose`

`sudo chmod +x /usr/local/bin/docker-compose`

`docker-compose version`

### git

`sudo apt update`

`sudo apt install git`

### clone the repo

`git clone https://github.com/stakwork/sphinx-swarm.git`

### aws:

create an A record like `*.swarmx.sphinx.chat` to the IP of the instance

### setup first time (only bitcoin):

in the root of sphinx-swarm directory, create a `.env`

copy the AWS creds into it, and `HOST=xxx`

`export ONLY_NODE=bitcoind`

`docker network create sphinx-swarm`

`docker-compose up -d`

`docker logs sphinx-swarm --follow`

### once bitcoind is synced

`docker stop sphinx-swarm && docker rm sphinx-swarm && docker-compose up sphinx-swarm -d && docker logs sphinx-swarm --follow`

### build docker

in app `yarn build`

docker build --no-cache -f src/bin/stack/Dockerfile -t sphinx-swarm . &&
docker tag sphinx-swarm sphinxlightning/sphinx-swarm:0.4.105 &&
docker push sphinxlightning/sphinx-swarm:0.4.105 &&
docker tag sphinx-swarm sphinxlightning/sphinx-swarm:latest &&
docker push sphinxlightning/sphinx-swarm:latest

### run this on new ec2 instance (debian)

```
curl -fsSL https://get.docker.com/ -o get-docker.sh &&
sh get-docker.sh &&
sudo usermod -aG docker $USER &&
sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) -o /usr/local/bin/docker-compose &&
sudo chmod +x /usr/local/bin/docker-compose &&
sudo apt update &&
sudo apt install git &&
git clone https://github.com/stakwork/sphinx-swarm.git &&
sudo docker network create sphinx-swarm
```

You can run this to setup the .env file (Not all .env options here)

```
cd sphinx-swarm &&
echo 'HOST=swarm<ENTER_NUMBER>.sphinx.chat' >> .env &&
echo 'NETWORK=bitcoin' >> .env &&
echo 'AWS_ACCESS_KEY_ID=<ENTER_AWS_ACCESS_KEY>' >> .env &&
echo 'AWS_SECRET_ACCESS_KEY=<ENTER_AWS_SECRET_KEY>' >> .env &&
echo 'AWS_REGION=us-east-1a' >> .env &&
echo 'AWS_S3_REGION_NAME=us-east-1' >> .env &&
echo 'STAKWORK_ADD_NODE_TOKEN=<ENTER_STAKWORK_TOKEN>' >> .env &&
echo 'STAKWORK_RADAR_REQUEST_TOKEN=<ENTER_STAKWORK_TOKEN>' >> .env &&
echo 'NO_REMOTE_SIGNER=true' >> .env &&
echo 'EXTERNAL_LND_MACAROON="<ENTER_LND_MACAROON>"' >> .env &&
echo 'EXTERNAL_LND_ADDRESS="<ENTER_LND_ADDRESS>"' >> .env &&
echo 'EXTERNAL_LND_CERT=<ENTER_EXTERNAL_LND_CERT>' >> .env &&
echo 'YOUTUBE_API_TOKEN=<ENTER_YOUTUBE_API_TOKEN>' >> .env &&
echo 'SWARM_UPDATER_PASSWORD=-' >> .env &&
echo 'JARVIS_FEATURE_FLAG_SCHEMA=true' >> .env
echo 'BACKUP_KEY=<BACKUP_KEY>' >> .env
echo 'FEATURE_FLAG_TEXT_EMBEDDINGS=true' >> .env
```
