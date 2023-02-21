<script lang="ts">
  import { Button, InlineLoading } from "carbon-components-svelte";
  import { createEventDispatcher } from "svelte";
  import Play from "carbon-icons-svelte/lib/Play.svelte";
  import Stop from "carbon-icons-svelte/lib/Power.svelte";
  import * as api from "../api";
  import { selectedNode, containers, node_state } from "../store";
  import type { Container } from "../api/swarm";

  let btnDis = false;

  const dispatch = createEventDispatcher();

  function sendStopEvent() {
    if (!$selectedNode.name) return;
    dispatch("stop_message", {
      text: $selectedNode.name,
    });
  }

  function sendStartEvent() {
    if (!$selectedNode.name) return;
    dispatch("start_message", {
      text: $selectedNode.name,
    });
  }

  async function listContainers() {
    const res: Container[] = await api.swarm.list_containers();

    containers.set(res);
  }

  async function startContainer(id) {
    btnDis = true;
    const res = await api.swarm.start_container(id);

    // Get new container state
    listContainers();

    // Send node started event to dashboard
    sendStartEvent();
    btnDis = false;
  }

  async function stopContainer(id) {
    btnDis = true;
    const res = await api.swarm.stop_container(id);

    // Get new container state
    listContainers();

    // Send node stopped event to dashboard
    sendStopEvent();
    btnDis = false;
  }
</script>

<aside class="node-action-wrap">
  {#if $node_state === "running"}
    <Button
      kind="secondary"
      disabled={btnDis}
      class="btn-stop"
      on:click={() => stopContainer(`${$selectedNode.name}.sphinx`)}
      iconDescription={`Stop ${$selectedNode.name}`}
      icon={Stop}
      size="field"
    />
  {:else if $node_state === "restarting"}
    <InlineLoading description={`Restarting ${$selectedNode.name}...`} />
  {:else}
    <Button
      kind="primary"
      disabled={btnDis}
      class="btn-start"
      on:click={() => startContainer(`${$selectedNode.name}.sphinx`)}
      iconDescription={`Start ${$selectedNode.name}`}
      icon={Play}
      size="field"
    />
  {/if}
</aside>

<style>
  .node-action-wrap {
    margin-left: 1rem;
  }
</style>
