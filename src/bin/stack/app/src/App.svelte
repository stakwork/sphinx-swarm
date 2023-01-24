<script lang="ts">
  import { selectedNode } from "./store";
  import { Loading } from "carbon-components-svelte";
  import Flow from "./Flow.svelte";
  import Controller from "./controls/Controller.svelte";
  import AddNode from "./nodes/AddNode.svelte";
  import NodeLogs from "./nodes/NodeLogs.svelte";
  import NodeUpdate from "./nodes/NodeUpdate.svelte";
  import { stack } from "./store";
  import { onMount } from "svelte";
  import * as api from "./api";
  import type { Stack } from "./nodes";

  async function getConfig() {
    const stackRemote: Stack = await api.swarm.get_config();
    if (stackRemote.nodes !== $stack.nodes) {
      stack.set(stackRemote);
    }
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

    <section class="header-btn-wrap">
      {#if $selectedNode}
        <NodeUpdate name={$selectedNode.name} version={$selectedNode.version} />
      {/if}

      {#if $selectedNode && $selectedNode.place === "Internal"}
        <NodeLogs nodeName={$selectedNode.name} />
      {/if}
    </section>

    <AddNode />
  </header>
  <div class="body">
    {#if $stack.nodes.length}
      <Flow />
    {:else}
      <div class="loader">
        <Loading />
      </div>
    {/if}
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
  .stack-title {
    color: white;
    margin-left: 0.5rem;
    font-size: 1.2rem;
  }
  .header-btn-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
  }
  .loader {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    align-items: center;
    justify-content: center;
    justify-items: center;
  }
</style>
