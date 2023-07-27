docker stop sphinx-swarm && docker rm sphinx-swarm

docker-compose up sphinx-swarm -d

docker logs sphinx-swarm --follow