<script lang="ts">
  import { selectedNode } from "./store";
  import Controls from "./Controls.svelte";
  import * as api from "./api";
  import { onMount } from "svelte";
  import Flow from "./Flow.svelte";

  async function getConfig() {
    const conf = await api.swarm.get_config();
    console.log(conf);
  }
  onMount(() => {
    getConfig();
  });

  $: console.log($selectedNode);
</script>

<main>
  <header>
    <div class="lefty logo-wrap">
      <img class="logo" alt="Sphinx icon" src="favicon.jpeg" />
    </div>
    {#if $selectedNode}
      <div class="title">{$selectedNode.name}</div>
    {/if}
  </header>
  <div class="body">
    <Flow />
    <Controls />
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
    background: #1A242E;
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
    width: 15rem;
    max-width: 15rem;
    height: 100%;
    border-right: 1px solid #101317;
  }
  .title {
    color: white;
    margin-left: 2rem;
    text-transform: capitalize;
  }
</style>
