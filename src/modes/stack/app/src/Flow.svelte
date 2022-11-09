<script lang="ts">
  import Svelvet from "svelvet";
  import { config, Node, NodeType } from "./nodes";
  import type { Node as SvelvetNode } from "svelvet";
  import { selectedNode } from "./store";

  const nodeCallback = (node) => {
    const n = config.nodes.find((n) => n.name === node.data.name);
    if (n) selectedNode.set(n);
  };

  export function toSvelvet(nodes: Node[], clickCallback): SvelvetNode[] {
    return nodes.map((n, i) => {
      return <SvelvetNode>{
        id: i + 1,
        position: { x: 140 * i + 25, y: 95 * i + 25 },
        width: 102,
        height: 120,
        borderRadius: 10,
        bgColor: colorz[n.type],
        clickCallback,
        data: { html: content(n.type), name: n.name },
      };
    });
  }

  function content(t: NodeType) {
    return `<section class='node-html'>
    <img src='swarm/${t.toLowerCase()}.png' class='node-img' width='50%'></img>
    <p style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>${t}</p>
  </section>`;
  }

  const colorz = {
    Btc: "#9D61FF",
    Lnd: "#D4A74E",
    Proxy: "#FF6161",
    Relay: "#49C998",
    Tribes: "#618AFF",
    Meme: "#9D61FF",
    Mqtt: "#660066",
    Auth: "#9D61FF",
    Postgres: "#9D61FF",
  };

  $: nodes = toSvelvet(config.nodes, nodeCallback);
</script>

<Svelvet
  {nodes}
  edges={[]}
  bgColor="#13181D"
  width={window.innerWidth}
  height={window.innerHeight}
  movement={false}
/>
