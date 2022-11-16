<script lang="ts">
  export let ctrls = [];
  import Ctrl from "./Ctrl.svelte";
  import { nodeStore, nodeConnections } from "./store";
  import { afterUpdate, beforeUpdate } from "svelte";
  let selectedId = "";

  beforeUpdate(() => {
    console.log("Node Store before ===", selectedId);
  });

  afterUpdate(() => {
    console.log("Node Store After ===", selectedId);
  });

  function getConnections(): any[] {
    if ($nodeStore === "Lnd") {
      return [
        {
          id: "conn1",
          text: "Btc",
        },
      ];
    }
    return [
      {
        id: "conn1",
        text: "Default",
      },
    ];
  }
</script>

<div class="controls">
  {#each ctrls as ctrl}
    {#if ctrl.usefor === "newnode" && ctrl.type === "dropdown"}
      <Ctrl {...ctrl} value={$nodeStore} {selectedId} on:input={() => alert("changed")} />
      <div class="spacer" />
    {:else if ctrl.usefor === "nodeconnections" && ctrl.type === "dropdown"}
      <Ctrl {...ctrl} value={$nodeConnections} items={getConnections()} />
      <div class="spacer" />
    {:else}
      <Ctrl {...ctrl} />
      <div class="spacer" />
    {/if}
  {/each}
</div>

<style>
  .spacer {
    margin-bottom: 1rem;
  }
  .controls {
    padding: 1.5rem;
  }
</style>
