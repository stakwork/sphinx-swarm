docker stop jarvis.sphinx
docker rm jarvis.sphinx
docker volume rm jarvis.sphinx

docker stop neo4j.sphinx
docker rm neo4j.sphinx
docker volume rm neo4j.sphinx

docker stop cln.sphinx
docker rm cln.sphinx
docker volume rm cln.sphinx

docker stop relay.sphinx
docker rm relay.sphinx
docker volume rm relay.sphinx

docker stop boltwall.sphinx
docker rm boltwall.sphinx
docker volume rm boltwall.sphinx

docker stop navfiber.sphinx
docker rm navfiber.sphinx
docker volume rm navfiber.sphinx

docker stop cache.sphinx
docker rm cache.sphinx
docker volume rm cache.sphinx

docker stop proxy.sphinx
docker rm proxy.sphinx
docker volume rm proxy.sphinx

docker stop sphinx-swarm
docker rm sphinx-swarm

docker stop load_balancer
docker rm load_balancer

sudo rm -rf $HOME/vol/stack