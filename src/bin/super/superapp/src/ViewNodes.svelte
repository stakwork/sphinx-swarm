<script lang="ts">
  import { ArrowLeft } from "carbon-icons-svelte";
  import { selectedNode } from "./store";
  import { onMount } from "svelte";
  import {
    get_child_swarm_config,
    get_child_swarm_containers,
  } from "../../../../../app/src/api/swarm";
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
  let containers = [];
  let message = "";
  let errorMessage = false;
  const sortedNodes = [];

  onMount(async () => {
    // get internal node for this service
    const result = await get_child_swarm_config({ host: $selectedNode });
    if (result.success && result.data.stack_error) {
      message = result.data.stack_error;
      errorMessage = true;
      loading = false;
      return;
    } else if (!result.success) {
      message = result.message;
      errorMessage = true;
      loading = false;
      return;
    }
    nodes = [...result.data];

    const swarm_containers = await get_child_swarm_containers({
      host: $selectedNode,
    });

    if (swarm_containers.success && swarm_containers.data.stack_error) {
      message = swarm_containers.data.stack_error;
      errorMessage = true;
      loading = false;
      return;
    } else if (!swarm_containers.success) {
      message = swarm_containers.message;
      errorMessage = true;
      loading = false;
      return;
    }
    containers = [...swarm_containers.data];
    console.log(containers);
    sortNodes();
    loading = false;
  });

  function findContainer(node_name: string) {
    for (let i = 0; i < containers.length; i++) {
      const container = containers[i];
      if (container.Names.includes(`/${node_name}.sphinx`)) {
        return container;
      }
    }
  }

  function sortNodes() {
    for (let i = 0; i < nodes.length; i++) {
      const node = nodes[i];
      if (node.place === "External") {
        continue;
      }
      const container = findContainer(node.name);
      sortedNodes.push({
        ...node,
        id: node.name,
        sn: `${i + 1}.`,
        version: node.version,
        name: `${node.name[0].toUpperCase()}${node.name.substring(1)}`,
        stop: container?.State,
      });
    }
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
      rows={sortedNodes}
      zebra
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
          {#if cell.value === "restarting"}
            <Button disabled={true}>Restarting...</Button>
          {:else if cell.value === "exited"}
            <Button>Start</Button>
          {:else}
            <Button>Stop</Button>
          {/if}
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
    overflow: auto;
    width: 100%;
    height: 100%;
  }
  .back {
    margin-bottom: 1rem;
  }

  .node_host {
    margin-bottom: 1rem;
  }
</style>
