<script lang="ts">
  import Svelvet from "svelvet";
  import { defaultPositions, type Stack } from "./nodes";
  import type { Node, NodeType } from "./nodes";
  import type { Node as SvelvetNode, Edge } from "svelvet";
  import { selectedNode } from "./store";
  import { onMount } from "svelte";

  export let stack: Stack = { network: "regtest", nodes: [] };

  const nodeCallback = (node) => {
    const n = stack.nodes.find((n) => n.name === node.data.name);
    if (n) {
      selectedNode.update((node) => (node && node.name === n.name ? null : n));
    }
  };

  $: flow = toSvelvet(stack.nodes, nodeCallback);

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
      const pos = defaultPositions[n.name] || [150, 150];
      return <SvelvetNode>{
        id: i + 1,
        position: { x: pos[0], y: pos[1] },
        width: 180,
        height: 90,
        borderRadius: 8,
        // bgColor: colorz[n.type],
        bgColor: "#1A242E",
        clickCallback,
        data: { html: content(n.type), name: n.name },
        sourcePosition: "right",
        targetPosition: "left",
        className: n.place === "Internal" ? "node-internal" : "node-external",
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

  function content(t: NodeType) {
    return `<section class='node-html'>
      <img src='swarm/${t.toLowerCase()}.png' class='node-img'></img>
      <p class="node-text">${t}</p>
    </section>`;
  }

  onMount(() => {
    () => {
      flow = toSvelvet(stack.nodes, nodeCallback);
    };
  });
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
