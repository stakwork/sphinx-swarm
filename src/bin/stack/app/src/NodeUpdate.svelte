<script lang="ts">
  import { Modal } from "carbon-components-svelte";
  import * as api from "./api";
  import { onDestroy, onMount } from "svelte";

  let open = false;
  export let name = "";
  export let version = "";
  let versions = [];

  function openModal() {
    open = true;
  }

  async function getImageVersions() {
    const nodeVersions = await api.swarm.get_node_images(`${name}.sphinx`);

    versions = nodeVersions.results;
  }

  onMount(() => {
    getImageVersions();
  })

  onDestroy(() => {
    versions = [];
  });
</script>

<section class="update-wrap">
  <button on:click={openModal} class="update-node-btn">
    <div class="title">{name}</div>
    {#if version}
      <div class="version">({version})</div>
    {/if}
  </button>

  <Modal
    bind:open
    modalHeading={`Update ${name} instance`}
    hasForm={true}
    class="get-logs-modal"
    size="sm"
    on:click:button--secondary={() => (open = !open)}
  >
    <section class="modal-content">
    </section>
  </Modal>
</section>

<style>
  .update-wrap {
    margin-left: 2rem;
  }
  .update-node-btn {
    background: transparent;
    border: 0;
    outline: 0;
    color: white;
    font-size: 1.15rem;
    cursor: pointer;
  }
  .version {
    color: white;
    margin: 0 1rem;
    font-weight: bold;
    font-size: 0.8rem;
  }
  .modal-content {
    padding: 0px 1.5rem;
  }
  .logs {
    background: #393939;
    width: 100%;
    height: 100%;
    max-height: 400px;
    overflow: auto;
    padding: 0.3rem 0.5rem;
    display: flex;
    flex-direction: column-reverse;
  }
  .log {
    color: white;
    margin: 1px 0;
    font-size: 0.8rem;
  }
</style>
