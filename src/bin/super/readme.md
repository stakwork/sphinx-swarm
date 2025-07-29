### build superadmin site

docker buildx build --platform linux/amd64 -f src/bin/super/Dockerfile -t sphinx-super --load .

docker tag sphinx-super sphinxlightning/sphinx-swarm-superadmin:0.1.52

docker push sphinxlightning/sphinx-swarm-superadmin:0.1.52

docker tag sphinx-super sphinxlightning/sphinx-swarm-superadmin:latest

docker push sphinxlightning/sphinx-swarm-superadmin:latest

### deploy

docker pull sphinxlightning/sphinx-swarm-superadmin
docker stop sphinx-swarm-superadmin
docker rm sphinx-swarm-superadmin
docker-compose -f superadmin.yml up sphinx-swarm-superadmin -d
docker logs sphinx-swarm-superadmin --follow
