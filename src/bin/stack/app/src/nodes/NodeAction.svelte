<script lang="ts">
  import { Button, InlineLoading } from "carbon-components-svelte";
  import Play from "carbon-icons-svelte/lib/Play.svelte";
  import Stop from "carbon-icons-svelte/lib/Power.svelte";
  import * as api from "../api";
  import { selectedNode, containers, node_state } from "../store";
  import type { Container } from "../api/swarm";

  let btnDis = false;

  async function listContainers() {
    const res: Container[] = await api.swarm.list_containers();

    containers.set(res);
  }

  async function startContainer(id) {
    btnDis = true;
    const res = await api.swarm.start_container(id);

    // Get new container state
    listContainers();
    btnDis = false;
  }

  async function stopContainer(id) {
    btnDis = true;
    const res = await api.swarm.stop_container(id);

    // Get new container state
    listContainers();
    btnDis = false;
  }
</script>

<aside class="node-action-wrap">
  {#if $node_state === "running"}
    <Button
      disabled={btnDis}
      class="btn-stop"
      on:click={() => stopContainer(`${$selectedNode.name}.sphinx`)}
      iconDescription={`Stop ${$selectedNode.name}`}
      icon={Stop}
      size="small"
    />
  {:else if $node_state === "restarting"}
    <InlineLoading description={`Restarting ${$selectedNode.name}...`} />
  {:else}
    <Button
      disabled={btnDis}
      class="btn-start"
      on:click={() => startContainer(`${$selectedNode.name}.sphinx`)}
      iconDescription={`Start ${$selectedNode.name}`}
      icon={Play}
      size="small"
    />
  {/if}
</aside>

<style>
  .node-action-wrap {
    margin-left: 20px;
  }
</style>
