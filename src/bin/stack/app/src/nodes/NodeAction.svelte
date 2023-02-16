<script lang="ts">
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

  $: stateClass =
    $node_state === "running"
      ? "btn-stop"
      : $node_state === "restarting"
      ? "btn-restart"
      : "btn-start";
</script>

<aside class="node-action-wrap">
  <button
    disabled={btnDis}
    on:click={() =>
      $node_state === "running"
        ? stopContainer(`${$selectedNode.name}.sphinx`)
        : startContainer(`${$selectedNode.name}.sphinx`)}
    class={`btn ${stateClass}`}
  >
    {#if $node_state === "running"}
      Stop
    {:else if $node_state === "restarting"}
      Restarting
    {:else}
      Start
    {/if}
  </button>
</aside>

<style>
  .node-action-wrap {
    margin-left: 20px;
  }
  .node-action-wrap .btn {
    padding: 6px 8px;
    border: 0px;
    outline: 0px;
    border-radius: 2px;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
  }

  .node-action-wrap .btn-stop {
    background-color: red;
  }
  .node-action-wrap .btn-start {
    background-color: green;
  }
  .node-action-wrap .btn-restart {
    background-color: gold;
  }
</style>
