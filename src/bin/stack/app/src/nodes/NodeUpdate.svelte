<script lang="ts">
  import {
    Modal,
    Button,
    StructuredList,
    StructuredListHead,
    StructuredListRow,
    StructuredListCell,
    StructuredListBody,
  } from "carbon-components-svelte";
  import * as api from "../api";
  import { onDestroy } from "svelte";
  import Upgrade from "carbon-icons-svelte/lib/Upgrade.svelte";
  import ImageRow from "./ImageRow.svelte";

  let open = false;

  export let name = "";
  export let version = "";
  let org = "";
  let repo = "";
  let loading = false;
  let page = 1;

  $: versionItems = [];

  let selected = "";

  function openModal() {
    open = true;

    clearData();
    getInitials();
  }

  function clearData() {
    versionItems = [];
    org = "";
    repo = "";
    page = 1;
  }

  function parseVersionData(nodeVersions) {
    return JSON.parse(nodeVersions.images).results;
  }

  function formatVersionData(versions) {
    return versions.map((v, i) => {
      return {
        id: i,
        name: v.name,
        last_updated: v.last_updated,
        status: v.tag_status,
        size: v.full_size,
      };
    });
  }

  async function getInitials() {
    loading = true;
    const nodeVersions = await api.swarm.get_node_images(name, page);
    const versions = parseVersionData(nodeVersions);

    org = nodeVersions.org;
    repo = nodeVersions.repo;

    const items = formatVersionData(versions);

    selected = `row-${version}-value`;

    versionItems = items;

    loading = false;
  }

  async function upgradeVersion() {
    console.log("UPDATE IMAGE");
  }

  onDestroy(() => {
    clearData();
  });

  let list;

  async function scrolled(el) {
    let obj = el.target;
    if (Math.ceil(obj.scrollTop) === obj.scrollHeight - obj.offsetHeight) {
      page += 1;
      const nodeVersions = await api.swarm.get_node_images(name, page);
      const newVersions = parseVersionData(nodeVersions);
      const items = newVersions ? formatVersionData(newVersions) : [];
      if (items.length) {
        versionItems = [...versionItems, ...items];
      }
    }
  }
</script>

<section class="update-wrap">
  <section class="update-node-btn">
    <div class="title">{name}</div>
    {#if version}
      <div class="version">({version})</div>
      <Button on:click={openModal} size="field" icon={Upgrade}>Update</Button>
    {/if}
  </section>

  <Modal
    bind:open
    modalHeading={`Update ${name} instance`}
    hasForm={true}
    class="get-logs-modal"
    primaryButtonText="Update instance"
    secondaryButtonText="Cancel"
    on:submit={upgradeVersion}
    on:click:button--secondary={() => (open = !open)}
  >
    <section class="modal-content">
      {#if loading}
        <div class="loading-wrap">
          <h5>Loading image versions .....</h5>
        </div>
      {:else}
        <div class="list" bind:this={list} on:scroll={scrolled}>
          <StructuredList selection {selected}>
            <StructuredListHead>
              <StructuredListRow head>
                <StructuredListCell head>Version</StructuredListCell>
                <StructuredListCell head>Last Updated</StructuredListCell>
                <StructuredListCell head>Status</StructuredListCell>
                <StructuredListCell head>{""}</StructuredListCell>
              </StructuredListRow>
            </StructuredListHead>
            <StructuredListBody>
              {#each versionItems as item}
                <ImageRow {item} {repo} />
              {/each}
            </StructuredListBody>
          </StructuredList>
        </div>
      {/if}
    </section>
  </Modal>
</section>

<style>
  .update-wrap {
    margin-left: 1.5rem;
  }
  .update-node-btn {
    background: transparent;
    border: 0;
    outline: 0;
    color: white;
    font-size: 1.15rem;
    cursor: pointer;
    display: flex;
    flex-direction: row;
    align-items: center;
  }
  .version {
    color: white;
    margin: 0 1rem;
    font-weight: bold;
    font-size: 0.8rem;
  }
  .modal-content {
    padding: 0px 1.5rem;
    /* width: 100%; */
  }

  .list {
    max-height: 400px;
    min-height: 400px;
    overflow-y: auto;
  }
</style>
