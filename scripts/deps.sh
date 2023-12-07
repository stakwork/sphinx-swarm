echo "INSTALLING DEPENDENCIES..."

curl -fsSL https://get.docker.com/ -o get-docker.sh

sh get-docker.sh

sudo usermod -aG docker $USER

sudo curl -L https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) -o /usr/local/bin/docker-compose

sudo chmod +x /usr/local/bin/docker-compose

docker-compose version

sudo apt update

sudo apt install git

