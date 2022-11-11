<script lang="ts">
  import { selectedNode } from "./store";
  import Ctrl from "./Ctrl.svelte";
  import { controls } from "./controls";
  import Users from "./Users.svelte";

  $: ctrls = $selectedNode && controls[$selectedNode.type];
  $: type = $selectedNode && $selectedNode.type;
</script>

{#if ctrls}
  <main>
    <header>
      {$selectedNode.name}
    </header>
    {#if type === "Relay"}
      <Users />
    {/if}
    <div class="controls">
      {#each ctrls as ctrl}
        <Ctrl {...ctrl} />
        <div class="spacer" />
      {/each}
    </div>
  </main>
{/if}

<style>
  header {
    margin-bottom: 1rem;
    text-transform: capitalize;
  }
  main {
    font-size: 2rem;
    height: 100vh;
    width: 23rem;
    border-radius: 0rem;
    position: fixed;
    right: 0rem;
    top: 4.1rem;
    background: #1A242E;
    padding: 2rem;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
  }
  .spacer {
    margin-bottom: 1rem;
  }
</style>
