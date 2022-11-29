<script lang="ts">
  import { selectedNode } from "./store";
  import * as api from "./api";
  import { onMount } from "svelte";
  import Flow from "./Flow.svelte";
  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import Controller from "./Controller.svelte";

  async function getConfig() {
    const conf = await api.swarm.get_config();
    console.log(conf);
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
    {/if}
    <section class="add-node-btn">
      <Button type="submit" size="field" icon={Add}>Add New Node</Button>
    </section>
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

  .add-node-btn {
    margin-left: auto;
    margin-right: 2.5rem;
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
  }
  .stack-title {
    color: white;
    margin-left: 0.5rem;
    font-size: 1.2rem;
  }
</style>
