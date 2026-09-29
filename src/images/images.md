### docker images

sphinxlightning/sphinx-proxy:latest

sphinxlightning/sphinx-boltwall:latest

sphinxlightning/cln-sphinx:latest

sphinxlightning/sphinx-relay-swarm:latest

sphinxlightning/sphinx-jarvis-backend:latest

sphinxlightning/sphinx-cache:latest

sphinxlightning/sphinx-lss:latest

sphinxlightning/sphinx-nav-fiber:latest

## advisor

`ghcr.io/stakwork/aws-advisor`, port 9034, volume `/data`. Added to the graph-mindset stack when `DEVOPS=1`. Private like neo4j: no Traefik route or public hostname, reachable only on the host's private IP (the UI exposes the account's inventory, probes, costs and decisions). Links: repo2graph (the agent), boltwall (its `stakwork_secret` becomes the advisor's repo2graph token), neo4j (the graph mirror). Secrets `api_token`, `mcp_token` and `callback_secret` are generated at stack creation and passed as env. The swarm env only seeds it, under an `ADVISOR_` prefix (`ADVISOR_AGENT_MODEL`, `ADVISOR_AGENT_API_KEY`, `ADVISOR_TYPESAFE_API_KEY`; the list is `ADVISOR_ENV` in `advisor.rs`): everything, schedules and graph included, is edited on the advisor's Settings page, where a saved value wins. The advisor can run on a different provider or key than repo2graph and never picks up another image's variables. No AWS credentials from the swarm: read-only access is configured in the advisor's Settings.

## browser

`ghcr.io/stakwork/strut-browser`, port 3000, built from `images/browser` in stakwork/strut: a Playwright server (`playwright run-server`) for the browser steps of repo2graph's workflows. Part of every stack that has a repo2graph, new or existing: `migrate_stack` adds the node, the link on repo2graph and the `auto_update` entry on the next start. Internal only: no Traefik route, no host port. Its one client is repo2graph, which gets `BROWSER_WS_URL=ws://browser.sphinx:3000` and `BROWSER_WS_PATH` through the link. The path is the secret half of the address (the server refuses a connection on any other), generated when the node is created and saved with the stack.

The browser is on a network of its own, `sphinx-browser`, and repo2graph joins that network as a second one. A page can reach whatever the browser's network reaches, so the browser is kept off `sphinx-swarm`, where neo4j, redis and the others answer by name. Because repo2graph is then on two networks, its Traefik labels name `sphinx-swarm` (`traefik.docker.network`). What the second network does not cover: a port another service publishes on the host is still reachable from the browser at the host's own address.

On `latest` and in `auto_update`. The server refuses a client whose Playwright major.minor differs from its own, so the image and repo2graph's `playwright-core` are released together. An existing repo2graph container gets the link's env and network when it is next recreated, which its next image update does.
