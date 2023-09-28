### build superadmin site

docker build --no-cache -f src/bin/super/Dockerfile -t sphinx-swarm-superadmin .

docker tag sphinx-swarm-superadmin sphinxlightning/sphinx-swarm-superadmin:0.1.5

docker push sphinxlightning/sphinx-swarm-superadmin:0.1.5

docker tag sphinx-swarm-superadmin sphinxlightning/sphinx-swarm-superadmin:latest

docker push sphinxlightning/sphinx-swarm-superadmin:latest
