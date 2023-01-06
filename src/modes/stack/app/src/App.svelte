<script lang="ts">
  import { selectedNode } from "./store";
  import Flow from "./Flow.svelte";
  import Controller from "./Controller.svelte";
  import AddNode from "./AddNode.svelte";
  import { onMount } from "svelte";
  import * as api from "./api";
  import { Button } from "carbon-components-svelte";
  import NodeLogs from "./NodeLogs.svelte";

  async function getConfig() {
    const conf = await api.swarm.get_config();
  }

  onMount(() => {
    getConfig();
  });
</script>

<main>
  <header>
    <div class="lefty logo-wrap">
      <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
      <span class="stack-title">Sphinx Stack</span>
    </div>

    {#if $selectedNode}
      <div class="title">{$selectedNode.name}</div>
      {#if $selectedNode.version}
        <div class="version">({$selectedNode.version})</div>
      {/if}
    {/if}

    {#if $selectedNode && $selectedNode.place === "Internal"}
      <NodeLogs nodeName={$selectedNode.name} />
    {/if}
    <AddNode />
  </header>
  <div class="body">
    <Flow />
    <Controller />
  </div>
</main>

<style>
  main {
    height: 100vh;
    width: 100vw;
    display: flex;
    flex-direction: column;
  }
  header {
    height: 4.2rem;
    min-height: 4.2rem;
    display: flex;
    background: #1a242e;
    align-items: center;
    border-bottom: 1px solid #101317;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
  }
  .logo-wrap {
    display: flex;
    align-items: center;
  }

  .logo-wrap .logo {
    width: 70px;
    padding: 12px;
    margin-left: 2.5rem;
  }
  .body {
    display: flex;
    height: 100%;
  }
  .lefty {
    width: 18rem;
    max-width: 18rem;
    height: 100%;
    border-right: 1px solid #101317;
  }
  .title {
    color: white;
    margin-left: 2rem;
    font-size: 1.15rem;
  }
  .version {
    color: white;
    margin: 0 1rem;
    font-weight: bold;
    font-size: 0.8rem;
  }
  .stack-title {
    color: white;
    margin-left: 0.5rem;
    font-size: 1.2rem;
  }
</style>
