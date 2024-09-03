<script lang="ts">
  import { ArrowLeft } from "carbon-icons-svelte";
  import { selectedNode } from "./store";
  import { onMount } from "svelte";
  import { get_child_swarm_config } from "../../../../../app/src/api/swarm";
  import {
    Button,
    DataTable,
    Toolbar,
    ToolbarContent,
    ToolbarSearch,
    Loading,
    ToolbarMenu,
    ToolbarMenuItem,
  } from "carbon-components-svelte";

  let loading = true;
  let selectedRowIds = [];
  let nodes = [];

  onMount(async () => {
    // get internal node for this service
    const result = await get_child_swarm_config({ host: $selectedNode });
    nodes = [...result.data];
    loading = false;
  });

  function nodeRow(node, index) {
    return {
      ...node,
      id: node.name,
      sn: `${index + 1}.`,
      name: `${node.name[0].toUpperCase()}${node.name.substring(1)}`,
    };
  }

  export let back = () => {};
</script>

<main>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="back" on:click={back}>
    <ArrowLeft size={32} />
  </div>
  <h2 class="node_host">{$selectedNode}</h2>
  {#if loading === true}
    <Loading />
  {:else}
    <DataTable
      headers={[
        { key: "sn", value: "S/N" },
        { key: "name", value: "Name" },
        // { key: "version", value: "Version" },
        { key: "upgrade", value: "Upgrade" },
        { key: "stop", value: "Stop/Start" },
      ]}
      selectable
      bind:selectedRowIds
      rows={nodes.map(nodeRow)}
    >
      <Toolbar>
        <ToolbarContent>
          <ToolbarSearch value="" shouldFilterRows />
          <ToolbarMenu disabled={false}>
            <ToolbarMenuItem>Restart all</ToolbarMenuItem>
            <ToolbarMenuItem hasDivider>API documentation</ToolbarMenuItem>
          </ToolbarMenu>
        </ToolbarContent>
      </Toolbar>
      <svelte:fragment slot="cell" let:row let:cell>
        {#if cell.key === "stop"}
          <Button>Stop</Button>
        {:else if cell.key === "upgrade"}
          <Button>Upgrade</Button>
        {:else}
          {cell.value}
        {/if}
      </svelte:fragment>
    </DataTable>
  {/if}
</main>

<style>
  main {
    padding: 2.5rem;
    width: 100%;
  }
  .back {
    margin-bottom: 1rem;
  }

  .node_host {
    margin-bottom: 1rem;
  }
</style>
