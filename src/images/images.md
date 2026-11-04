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

`ghcr.io/stakwork/aws-advisor`, port 9034 (UI and API) and 9035 (the doorman: waiting page and wake-on-traffic proxy for parked instances, `DOORMAN_PORT`), volume `/data`. Added to the graph-mindset stack when `DEVOPS=1`. Private like neo4j: no Traefik route or public hostname, reachable only on the host's private IP (the UI exposes the account's inventory, probes, costs and decisions). Links: repo2graph (the agent), boltwall (its `stakwork_secret` becomes the advisor's repo2graph token), neo4j (the graph mirror). Secrets `api_token`, `mcp_token` and `callback_secret` are generated at stack creation and passed as env. The swarm env only seeds it, under an `ADVISOR_` prefix (`ADVISOR_AGENT_MODEL`, `ADVISOR_AGENT_API_KEY`, `ADVISOR_TYPESAFE_API_KEY`; the list is `ADVISOR_ENV` in `advisor.rs`): everything, schedules and graph included, is edited on the advisor's Settings page, where a saved value wins. The advisor can run on a different provider or key than repo2graph and never picks up another image's variables. No AWS credentials from the swarm: read-only access is configured in the advisor's Settings.

## browser

`ghcr.io/stakwork/strut-browser`, port 3000, built from `images/browser` in stakwork/strut: a Playwright server (`playwright run-server`) for the browser steps of repo2graph's workflows. Part of every stack that has a repo2graph, new or existing: `migrate_stack` adds the node, the link on repo2graph and the `auto_update` entry on the next start. Internal only: no Traefik route, no host port. Its one client is repo2graph, which gets `BROWSER_WS_URL=ws://browser.sphinx:3000` and `BROWSER_WS_PATH` through the link. The path is the secret half of the address (the server refuses a connection on any other), generated when the node is created and saved with the stack.

The browser is on a network of its own, `sphinx-browser`, and repo2graph joins that network as a second one. A page can reach whatever the browser's network reaches, so the browser is kept off `sphinx-swarm`, where neo4j, redis and the others answer by name. Because repo2graph is then on two networks, its Traefik labels name `sphinx-swarm` (`traefik.docker.network`). What the second network does not cover: a port another service publishes on the host is still reachable from the browser at the host's own address.

On `latest` and in `auto_update`. The server refuses a client whose Playwright major.minor differs from its own, so the image and repo2graph's `playwright-core` are released together. An existing repo2graph container gets the link's env and network when it is next recreated, which its next image update does.

## voice

`sphinxlightning/sphinx-voice`, port 8080, run as `node dist/one-to-one.js`: the member ("1:1") mode of sphinx-voice, where a member talks to the org's agent (Jamie) by voice from the Peel app. The same image as meeting-mode Jamie, whose own command is replaced. Internal only: no Traefik route, no host port.

- **Strut calls it.** A member's `POST /lab/voice/calls` reaches repo2graph's strut, which forwards it to `http://voice.sphinx:8080/calls` with the member as `x-strut-actor`. repo2graph gets `STRUT_VOICE_URL` and `STRUT_VOICE_TOKEN` through a `voice` link. The token is the secret the two share, generated when the node is created and saved with the stack.
- **It calls strut back.** Every turn runs the orchestrator as the member, on `STRUT_URL=http://repo2graph.sphinx:3355/lab`. Its credential, `STRUT_API_KEY`, is boltwall's `stakwork_secret`, which is repo2graph's `API_TOKEN`. Strut posts run ends to `CALLBACK_URL_BASE=http://voice.sphinx:8080`.
- **It connects out to LiveKit.** It joins each call's room on the central LiveKit, `wss://livekit.sphinx.chat`. `LIVEKIT_API_KEY` and `LIVEKIT_API_SECRET` come from the swarm's `.env`, and so does `LIVEKIT_URL` when it is not the central one.
  - The super writes the keys into a new swarm's `.env` when they are set in its own (`LIVEKIT_ENV` in `src/bin/super/util.rs`).
  - An operator sets them on a running swarm through the super's env update.
  - The node exists only once the keys are there: `second_brain_imgs` adds it to a new stack, and `migrate_stack` adds the node, the link on repo2graph and the `auto_update` entry on the next start.
- **Optional `.env` values.** `VOICE_MODEL` is the model a voice turn asks for; unset, the service's default `haiku` stands, which the swarm's gateway allows (every user's virtual key allows all models of its providers). `STRUT_TTS_VOICE` is the voice a call starts in.

On `latest` and in `auto_update`, like repo2graph: the two speak strut's contract (the orchestrator turn, `/audio/*`, `/voice/calls`). The image is published for linux/amd64 only. Nothing is kept, so there is no backup.

## maple

`ghcr.io/mapleprivacylabs/maple-proxy`, port 8080: Maple's OpenAI-compatible proxy for the open models Maple (maple.ai) serves from attested enclaves, so strut can run a turn on a private model (`maple/gemma4-31b`). It attests Maple's AWS Nitro enclave (`https://enclave.trymaple.ai`, the image's default) and encrypts every request to it. Upstream docs: https://github.com/MaplePrivacyLabs/Maple/blob/master/proxy/README.md.

- **It reads every prompt in plaintext.** So it sits beside strut on the swarm network: internal only, no Traefik route, no host port, CORS off.
- **Strut calls it.** repo2graph gets `MAPLE_BASE_URL=http://maple.sphinx:8080/v1` and `MAPLE_API_KEY` through a `maple` link, and sends the key on every request as the bearer. The proxy itself holds no key.
- **The key gates the node.** `MAPLE_API_KEY` comes from the swarm's `.env`. The super writes it into a new swarm's `.env` when it is set in its own (`MAPLE_ENV` in `src/bin/super/util.rs`), and keeps it when a warm swarm is nuked. `second_brain_imgs` adds the node to a new stack, and `migrate_stack` adds the node and the link on repo2graph on the next start.
- **Its cache secret.** `MAPLE_CACHE_NAMESPACE_ROOT` keeps the provider's prompt cache across proxy restarts. It is 32 random bytes in padded base64 (the proxy refuses to start on anything else), generated when the node is created, saved with the stack and blanked by `remove_tokens`.
- **The voice model.** The voice service's `VOICE_MODEL` stays `haiku` by default. Set `VOICE_MODEL=maple/gemma4-31b` in the `.env` once the swarm's strut has the `maple` provider.

Pinned (`MAPLE_PROXY_VERSION`, 0.4.5) and not in `auto_update`: third-party code that holds plaintext and decides whether the enclave is genuine. Maple's exact version tags are immutable; `latest`, `0.4` and `0` move. Upgrade on purpose: bump the constant for new stacks, and update a running swarm's node to the new version. Read-only root filesystem, every capability dropped, no-new-privileges, 256 MB of memory. Published for linux/amd64 and linux/arm64. Nothing is kept, so there is no backup.
