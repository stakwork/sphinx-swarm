<script lang="ts">
  import {
    Modal,
    Button,
    StructuredList,
    StructuredListHead,
    StructuredListRow,
    StructuredListCell,
    StructuredListBody,
    StructuredListInput,
  } from "carbon-components-svelte";
  import * as api from "./api";
  import { onDestroy, onMount, afterUpdate } from "svelte";
  import Upgrade from "carbon-icons-svelte/lib/Upgrade.svelte";
  import CheckmarkFilled from "carbon-icons-svelte/lib/CheckmarkFilled.svelte";

  let open = false;

  export let name = "";
  export let version = "";
  let org = "";
  let repo = "";
  let loading = false;

  let versionItems = [];
  $: selected = "row-0-value";

  function openModal() {
    open = true;

    clearData();
    getVersions();
  }

  function clearData() {
    versionItems = [];
    org = "";
    repo = "";
  }

  async function getVersions() {
    const nodeVersions = await api.swarm.get_node_images(name);
    const versions = JSON.parse(nodeVersions.images).results;

    org = nodeVersions.org;
    repo = nodeVersions.repo;

    versionItems = versions.map((v, i) => {
      return {
        id: i,
        name: v.name,
        last_updated: v.last_updated,
        status: v.tag_status,
        size: v.full_size
      };
    });
  }

  async function upgradeVersion() {}

  onDestroy(() => {
    clearData();
  });

  function typeSelected() {
    name = "";
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
            <StructuredListRow label for="row-{item.id}">
              <StructuredListCell>{repo}@{item.name}</StructuredListCell>
              <StructuredListCell>{item.last_updated}</StructuredListCell>
              <StructuredListCell>
                {item.status}
              </StructuredListCell>
              <StructuredListInput
                id="row-{item.id}"
                value="row-{item.id}-value"
                title="row-{item.id}-title"
                name="row-{item.id}-name"
              />
              <StructuredListCell>
                <CheckmarkFilled
                  class="bx--structured-list-svg"
                  aria-label="select an option"
                  title="select an option"
                />
              </StructuredListCell>
            </StructuredListRow>
          {/each}
        </StructuredListBody>
      </StructuredList>
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
  }
</style>
