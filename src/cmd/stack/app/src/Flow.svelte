<script lang="ts">
  export let fixed = false;

  import Drawflow from "drawflow";
  import { onMount } from "svelte";
  import nodes, { Node } from "./nodes";
  import { selectedNode } from "./store";

  function click(e) {
    // console.log(e);
    e.path.forEach((el) => {
      if (!el.className) return;
      const classes: string[] = el.className.split(" ");
      if (classes.includes("drawflow-node")) {
        const lastClass = classes[classes.length - 1];
        const wordz = lastClass.split("-");
        const name = wordz[classes.length - 1];
        const node = nodes.find((n) => n.name === name);
        selectedNode.set(node);
      }
    });
  }

  function content(n: Node) {
    if (!n.data) {
      return `<div class="content-title-simple">${n.name}</div>`;
    }
    return `<div>
        <div class="content-title">${n.name}</div>
        <div class="content-body">${n.data}</div>
      </div>`;
  }

  function setConnectionClasses() {
    const els = Array.from(
      document.getElementsByClassName("connection input_1")
    );
    console.log(els);
    let i = 0;
    nodes.forEach((n) => {
      n.outs.forEach((out) => {
        let path = els[i] && (els[i].firstChild as any);
        if (path) path.classList.add(`conn-${n.type}`);
        i++;
      });
    });
  }

  onMount(() => {
    const id = document.getElementById("drawflow");
    const editor = new Drawflow(id);
    if (fixed) editor.editor_mode = "fixed";
    editor.start();
    nodes.forEach((n) => {
      const ins = nodes.filter((nn) => nn.outs.includes(n.name));
      editor.addNode(
        n.name,
        ins.length,
        n.outs.length,
        n.x,
        n.y,
        `flow-${n.type} flow-name-${n.name}`,
        { name: n.name },
        content(n),
        false
      );
    });
    nodes.forEach((n, i) => {
      n.outs.forEach((c, ii) => {
        const ids = editor.getNodesFromName(c);
        if (ids.length) {
          editor.addConnection(i + 1, ids[0], `output_${ii + 1}`, `input_1`);
        }
      });
    });
    editor.on("mouseMove", function (pos) {
      // console.log("mouse: " + pos.x, pos.y);
    });
    editor.on("click", function (e) {
      console.log(e);
    });
    window.setTimeout(() => {
      setConnectionClasses();
    }, 2);
  });
</script>

<div
  id="drawflow"
  on:click={click}
  style="height:calc(100vh - 4.2rem);width:100vw;"
/>
