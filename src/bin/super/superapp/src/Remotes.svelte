<script lang="ts">
  import {
    Button,
    DataTable,
    Toolbar,
    ToolbarContent,
    ToolbarSearch,
    ToolbarMenu,
    ToolbarMenuItem,
  } from "carbon-components-svelte";
  import Healthcheck from "./Healthcheck.svelte";
  import UploadIcon from "carbon-icons-svelte/lib/Upload.svelte";
  import Tribes from "./Tribes.svelte";
  import * as api from "../../../../../app/src/api";
  import { remotes, tribes } from "./store";
  import { onMount } from "svelte";
  import type { Remote } from "./types/types";
  import { splitHost } from "./utils/index";

  let selectedRowIds = [];

  async function getConfig() {
    const conf = await api.swarm.get_config();
    if (conf && conf.stacks && conf.stacks.length) {
      remotes.set(conf.stacks);
      const serverTribes = await getTribes(conf.stacks);
      tribes.set(serverTribes);
    }
  }
  onMount(() => {
    getConfig();
  });

  function something() {
    console.log("something");
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
        <Button kind="tertiary" on:click={something} icon={UploadIcon}>
          Do something
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
</main>

<style>
  main {
    overflow: auto;
    max-height: var(--body-height);
    width: 100%;
  }
</style>
