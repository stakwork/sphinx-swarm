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
