<script lang="ts">
  import { Modal, Dropdown } from "carbon-components-svelte";
  import * as api from "./api";
  import { onDestroy, onMount } from "svelte";

  let open = false;
  let tag = "";

  export let name = "";
  export let version = "";

  let versions = [];

  let versionItems = versions.length ? versions.map((v) => ({
    id: v.name,
    text: v.name,
  })) : [{id: "", text: ""}];

  function openModal() {
    open = true;
  }

  async function getVersions() {
    const nodeVersions = await api.swarm.get_node_images(name);

    versions = nodeVersions.results;
  }

  async function upgradeVersion() {
    const nodeVersions = await api.swarm.get_node_images(name);

    versions = nodeVersions.results;
  }

  onMount(() => {
    getVersions();
  });

  onDestroy(() => {
    versions = [];
  });

  function typeSelected() {
    name = "";
  }
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
    primaryButtonText="Update instance"
    secondaryButtonText="Cancel"
    on:submit={upgradeVersion}
    on:click:button--secondary={() => (open = !open)}
  >
    <section class="modal-content">
      <Dropdown
        titleText="Versions"
        bind:selectedId={tag}
        items={versionItems}
      />
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
