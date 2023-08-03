<script lang="ts">
  import Svelvet from "svelvet";
  import { defaultPositions } from "./nodes";
  import { chipSVG, type Node, type NodeType, smalls } from "./nodes";
  import type { Node as SvelvetNode, Edge } from "svelvet";
  import {selectedNode, stack} from "./store";

  $: flow = toSvelvet($stack.nodes, nodeCallback);

  const nodeCallback = (node) => {
    if (!$stack.ready) return console.log("stack is not ready...");
    const n = $stack.nodes.find((n) => n.name === node.data.name);
    if (n) {
      console.log("=>", n);
      selectedNode.update((node) => (node && node.name === n.name ? null : n));
    }
  };

  function toSvelvet(
    ns: Node[],
    clickCallback
  ): { edges: Edge[]; nodes: SvelvetNode[] } {
    const edges: Edge[] = [];
    const nodes = ns.map((n, i) => {
      if (n.links && n.links.length) {
        n.links.forEach((link) => {
          const idx = ns.findIndex((node) => node.name === link);
          if (idx > -1)
            edges.push({
              id: `edge-${i + 1}-${idx + 1}`,
              source: idx + 1,
              target: i + 1,
              edgeColor: "#dddddd",
              // noHandle: true,
              type: ns[idx].place === "Internal" ? "bezier" : "straight",
              animate: ns[idx].place === "External" || n.type === "Traefik",
            });
        });
      }

      const pos = defaultPositions()[n.name] || [150, 150];

      const remoteHsmd = n.plugins && n.plugins.includes("HsmdBroker");

      const isSmall = smalls.includes(n.name);

      let className = `node-${n.name}`;
      if (n.place === "Internal") className += " node-internal";
      else className += " node-external";
      if (isSmall) className += " node-small";

      return <SvelvetNode>{
        id: i + 1,
        position: { x: pos[0], y: pos[1] },
        width: isSmall ? 140 : 180,
        height: isSmall ? 70 : 90,
        borderRadius: 8,
        // bgColor: colorz[n.type],
        bgColor: "#1A242E",
        clickCallback,
        data: { html: content(n.type, remoteHsmd), name: n.name },
        sourcePosition: "right",
        targetPosition: "left",
        className,
      };
    });
    return { nodes, edges };
  }

  const colorz = {
    Btc: "#D4A74E",
    Lnd: "#9D61FF",
    Proxy: "#FF6161",
    Relay: "#49C998",
    Tribes: "#618AFF",
    Meme: "#660066",
    Mqtt: "#660066",
    Auth: "#9D61FF",
    Postgres: "#9D61FF",
  };

  function content(t: NodeType, remoteHsmd = false) {
    return `<section class='node-html'>
      <img src='swarm/${t.toLowerCase()}.png' class='node-img'></img>
      <p class="node-text">${t}</p>
      ${
        remoteHsmd
          ? `<div class="remote-hsmd">${chipSVG}</div>`
          : "<span></span>"
      }
    </section>`;
  }
</script>

<Svelvet
  nodes={flow.nodes}
  edges={flow.edges}
  bgColor="#101317"
  width={window.innerWidth}
  height={window.innerHeight}
  initialLocation={{ x: window.innerWidth / 2, y: window.innerHeight / 2 }}
  movement={true}
/>
