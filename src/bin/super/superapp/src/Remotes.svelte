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
  import * as api from "../../../../../app/src/api";
  import { remotes, type Remote } from "./store";
  import { onMount } from "svelte";

  let selectedRowIds = [];

  async function getConfig() {
    const conf = await api.swarm.get_config();
    if (conf && conf.stacks && conf.stacks.length) {
      remotes.set(conf.stacks);
    }
  }
  onMount(() => {
    getConfig();
  });

  function something() {
    console.log("something");
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
