<script lang="ts">
  import Flow from "./Flow.svelte";
  import { selectedNode } from "./store";
  import Controls from "./Controls.svelte";
  import * as api from "./api";
  import { onMount } from "svelte";

  onMount(() => {
    api.swarm.get_config();
  });

  $: console.log($selectedNode);
</script>

<main>
  <header>
    <div class="lefty logo-wrap">Sphinx Stack</div>
    {#if $selectedNode}
      <div class="title">{$selectedNode.name}</div>
    {/if}
  </header>
  <div class="body">
    <Flow fixed />
    <Controls />
  </div>
</main>

<style>
  main {
    height: 100vh;
    width: 100vw;
    display: flex;
    background: #161616;
    flex-direction: column;
  }
  header {
    height: 4.2rem;
    display: flex;
    align-items: center;
    border-bottom: 1px solid #bfbfbf;
  }
  .logo-wrap {
    display: flex;
    align-items: center;
  }
  .body {
    display: flex;
    height: 100%;
  }
  .lefty {
    width: 15rem;
    max-width: 15rem;
    height: 100%;
    border-right: 1px solid #bfbfbf;
  }
  .title {
    color: white;
    margin-left: 2rem;
  }
</style>
