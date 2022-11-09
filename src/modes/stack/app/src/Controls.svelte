<script lang="ts">
  import { selectedNode } from "./store";
  import Ctrl from "./Ctrl.svelte";
  import { controls } from "./controls";

  $: ctrls = $selectedNode && controls[$selectedNode.type];
</script>

{#if ctrls}
  <main>
    <header>
      {$selectedNode.name}
    </header>
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
  }
  main {
    font-size: 2rem;
    height: 80vh;
    width: 23rem;
    border: 1px solid #bfbfbf;
    border-radius: 1rem;
    position: fixed;
    right: 2rem;
    top: 6rem;
    background: #161616;
    padding: 2rem;
  }
  .spacer {
    margin-bottom: 1rem;
  }
</style>
