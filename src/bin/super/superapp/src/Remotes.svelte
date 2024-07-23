<script lang="ts">
  import {
    Button,
    DataTable,
    Toolbar,
    ToolbarContent,
    ToolbarSearch,
    Modal,
    TextInput,
  } from "carbon-components-svelte";
  import Healthcheck from "./Healthcheck.svelte";
  import UploadIcon from "carbon-icons-svelte/lib/Upload.svelte";
  import Tribes from "./Tribes.svelte";
  import * as api from "../../../../../app/src/api";
  import { remotes, tribes } from "./store";
  import { onMount } from "svelte";
  import type { Remote } from "./types/types";
  import { splitHost } from "./utils/index";

  let open = false;
  let new_host = "";
  let new_description = "";
  let new_instance = "";

  let selectedRowIds = [];

  async function getConfig() {
    const conf = await api.swarm.get_config();
    if (conf && conf.stacks && conf.stacks.length) {
      remotes.set(conf.stacks);
      const serverTribes = await getTribes(conf.stacks);
      tribes.set(serverTribes);
    }
  }

  async function getConfigSortByUnhealthy() {
    const conf = await api.swarm.get_config();
    if (conf && conf.stacks && conf.stacks.length) {
      let unhealthyRemotes = [];
      let healthyRemotes = [];
      for (let i = 0; i < conf.stacks.length; i++) {
        let el = conf.stacks[i];
        const host = el.host;
        try {
          let url = `https://boltwall.${host}/stats`;
          // custom URLs
          if (!url.includes("swarm")) {
            url = `https://${host}/api/stats`;
          }
          console.log("URL", url);
          const r = await fetch(url);
          await r.json();
          healthyRemotes.push(el);
        } catch (e) {
          console.warn(e);
          unhealthyRemotes.push(el);
        }
      }

      remotes.set([...unhealthyRemotes, ...healthyRemotes]);
      const serverTribes = await getTribes([
        ...unhealthyRemotes,
        ...healthyRemotes,
      ]);
      tribes.set(serverTribes);
    }
  }

  onMount(() => {
    getConfig();

    const interval = setInterval(getConfigSortByUnhealthy, 3000);
    getConfigSortByUnhealthy();
    return () => clearInterval(interval);
  });

  function openAddSwarmModal() {
    open = true;
  }

  async function getTribes(r: Remote[]) {
    const hostPrefixes = [];
    for (let i = 0; i < r.length; i++) {
      const hostPrefix = splitHost(r[i].host);
      if (hostPrefix) {
        hostPrefixes.push(hostPrefix);
      }
    }
    //Get all tribes that belong to Swarm
    return await getAllTribeFromTribeHost(hostPrefixes.join());
  }

  async function getAllTribeFromTribeHost(swarms) {
    try {
      const r = await fetch(
        `https://tribes.sphinx.chat/tribes/app_urls/${swarms}`
      );
      const j = await r.json();
      console.log(j);
      return j;
    } catch (e) {
      console.warn(e);
      return {};
    }
  }

  function remoterow(r: Remote) {
    return { ...r, id: r.host };
  }

  function handleSubmitAddSwarm() {
    const data = {
      host: new_host,
      intance: new_instance,
      description: new_description,
    };

    console.log(data);
    //send data to backened
    //get config again
    //clear host, instance, description
    //add notification for success
  }
</script>

<main>
  <DataTable
    headers={[
      { key: "host", value: "Host" },
      { key: "note", value: "Description" },
      { key: "ec2", value: "Instance" },
      { key: "tribes", value: "Tribes" },
      { key: "health", value: "Health" },
    ]}
    rows={$remotes.map(remoterow)}
    selectable
    bind:selectedRowIds
  >
    <Toolbar>
      <ToolbarContent>
        <ToolbarSearch value="" shouldFilterRows />
        <!-- <ToolbarMenu disabled={false}>
            <ToolbarMenuItem>Restart all</ToolbarMenuItem>
            <ToolbarMenuItem hasDivider>API documentation</ToolbarMenuItem>
            <ToolbarMenuItem hasDivider>Stop all</ToolbarMenuItem>
          </ToolbarMenu> -->
        <Button kind="tertiary" on:click={openAddSwarmModal} icon={UploadIcon}>
          Add New Swarm
        </Button>
      </ToolbarContent>
    </Toolbar>
    <svelte:fragment slot="cell" let:row let:cell>
      {#if cell.key === "health"}
        <Healthcheck host={row.id} />
      {:else if cell.key === "tribes"}
        <Tribes host={row.id} />
      {:else}
        {cell.value}
      {/if}
    </svelte:fragment>
  </DataTable>

  <Modal
    bind:open
    modalHeading="Add new Swarm"
    primaryButtonText="Confirm"
    secondaryButtonText="Cancel"
    selectorPrimaryFocus="#db-name"
    on:click:button--secondary={() => (open = false)}
    on:open
    on:close
    on:submit={handleSubmitAddSwarm}
  >
    <p>Add a new swarm to the list of swarms.</p>
    <div class="text_input_container">
      <TextInput
        id="host"
        labelText="Host"
        placeholder="Enter Swarm Host..."
        bind:value={new_host}
      />
    </div>
    <div class="text_input_container">
      <TextInput
        id="description"
        labelText="Description"
        placeholder="Enter Swarm Description..."
        bind:value={new_description}
      />
    </div>
    <div class="text_input_container">
      <TextInput
        id="instance"
        labelText="Instance"
        placeholder="Enter Swarm Instance Size..."
        bind:value={new_instance}
      />
    </div>
  </Modal>
</main>

<style>
  main {
    overflow: auto;
    max-height: var(--body-height);
    width: 100%;
  }

  .text_input_container {
    margin-top: 1rem;
  }
</style>
