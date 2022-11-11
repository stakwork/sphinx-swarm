<script lang="ts">
  import { selectedNode } from "./store";
  import Ctrl from "./Ctrl.svelte";
  import { controls } from "./controls";
  import RelayControls from "./RelayControls.svelte";

  $: type = $selectedNode && $selectedNode.type;
  $: ctrls = $selectedNode && controls[type];
</script>

{#if ctrls}
  <main>
    <header>
      <img src={`swarm/${type}.png`} class="node-top-img" alt="node " />
      {$selectedNode.name}
    </header>
    {#if type === "Relay"}
      <RelayControls />
    {:else}
      <div class="controls">
        {#each ctrls as ctrl}
          <Ctrl {...ctrl} />
          <div class="spacer" />
        {/each}
      </div>
    {/if}
  </main>
{/if}

<style>
  header {
    margin-bottom: 1rem;
    text-transform: capitalize;
    font-size: 1.15rem;
    display: flex;
    align-items: center;
  }

  header .node-top-img {
    width: 1.25rem;
    margin-right: 15px;
  }
  main {
    font-size: 2rem;
    height: 100vh;
    width: 25rem;
    border-radius: 0rem;
    position: fixed;
    right: 0rem;
    top: 4.14rem;
    background: #1a242e;
    padding: 2rem;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
  }
  .spacer {
    margin-bottom: 1rem;
  }
</style>
