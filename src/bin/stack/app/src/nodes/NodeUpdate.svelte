<script lang="ts">
  import {
    Modal,
    Button,
    StructuredList,
    StructuredListHead,
    StructuredListRow,
    StructuredListCell,
    StructuredListBody,
    Loading,
  } from "carbon-components-svelte";
  import * as api from "../api";
  import { onDestroy } from "svelte";
  import Upgrade from "carbon-icons-svelte/lib/Upgrade.svelte";
  import ImageRow from "./ImageRow.svelte";
  import { selectedNode } from "../store";

  let open = false;

  $: name = $selectedNode.name;
  let selectedVersion = $selectedNode.version;
  $: btnDis = false;

  let org = "";
  let repo = "";
  let loading = false;
  let scrollLoading = false;
  let hasMore = true;
  let page = 1;

  $: versionItems = [];

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
    try {
      return JSON.parse(nodeVersions.images).results;
    } catch (e) {
      return [];
    }
  }

  function formatVersionData(versions) {
    return versions?.map((v, i) => {
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
    if (!nodeVersions) return;
    const versions = parseVersionData(nodeVersions);

    org = nodeVersions.org;
    repo = nodeVersions.repo;

    versionItems = formatVersionData(versions);
    loading = false;
  }

  async function upgradeVersion() {
    if (name && selectedVersion) {
      btnDis = true;
      // console.log("update =>", name, selectedVersion);
      await api.swarm.update_node(name, selectedVersion);
      btnDis = false;
    }
  }

  onDestroy(() => {
    clearData();
  });

  async function scrolled(el) {
    let obj = el.target;
    if (Math.ceil(obj.scrollTop) === obj.scrollHeight - obj.offsetHeight) {
      page += 1;
      scrollLoading = true;

      const nodeVersions = await api.swarm.get_node_images(name, page);
      const newVersions = parseVersionData(nodeVersions);
      const items = newVersions ? formatVersionData(newVersions) : [];

      if (items.length) {
        versionItems = [...versionItems, ...items];
        scrollLoading = false;
      } else {
        hasMore = false;
      }
    }
  }

  function listChange(e) {
    const details = e.detail.split("-");
    if (details.length > 1) selectedVersion = details[1];
  }
</script>

<section class="update-wrap">
  <section class="update-node-btn">
    <div class="title">{name}</div>
    {#if $selectedNode.version}
      <div class="version">{`(${$selectedNode.version})`}</div>
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
    disabled={btnDis}
    primaryButtonDisabled={btnDis}
    on:submit={upgradeVersion}
    on:click:button--secondary={() => (open = !open)}
  >
    {#if btnDis}
      <div class="overlay">
        <Loading />
      </div>
    {/if}
    <section class="modal-content">
      {#if loading}
        <div class="loading-wrap">
          <h5>Loading image versions .....</h5>
        </div>
      {:else}
        <div class="list" on:scroll={scrolled}>
          <StructuredList
            on:change={listChange}
            selection
            selected={`row-${selectedVersion}-value`}
          >
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
          {#if scrollLoading && hasMore}
            <p class="scroll-msg">Loading more ...</p>
          {:else}
            <p class="scroll-msg">No more data</p>
          {/if}
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
  .title {
    color: white;
    font-weight: bold;
    font-size: 1rem;
  }
  .version {
    color: white;
    margin: 0 1rem;
    font-size: 0.8rem;
  }
  .modal-content {
    padding: 0px 1.5rem;
    width: 100%;
  }
  .list {
    max-height: 400px;
    min-height: 400px;
    min-width: 100%;
    overflow-y: auto;
  }
  .scroll-msg {
    text-align: center;
    padding: 0;
    margin: 0;
    margin-top: -50px;
  }
  .overlay {
    min-width: 100%;
    min-height: 100%;
    z-index: 100;
    background-color: rgb(211, 211, 211, 0.1);
  }
</style>
