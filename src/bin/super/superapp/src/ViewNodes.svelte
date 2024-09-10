<script lang="ts">
  import { ArrowLeft } from "carbon-icons-svelte";
  import { selectedNode } from "./store";
  import { onMount } from "svelte";
  import {
    get_child_swarm_config,
    get_child_swarm_containers,
    start_child_swarm_containers,
    stop_child_swarm_containers,
    update_child_swarm_containers,
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
    ToastNotification,
  } from "carbon-components-svelte";

  let loading = true;
  let selectedRowIds = [];
  let nodes = [];
  let containers = [];
  let message = "";
  let errorMessage = false;
  $: sortedNodes = [];
  $: nodes_to_be_modified = [];
  let show_notification = false;

  async function setupNodes() {
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
    sortNodes();
    loading = false;
  }

  onMount(async () => {
    // get internal node for this service
    setupNodes();
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
    const tempSortedNodes = [];
    for (let i = 0; i < nodes.length; i++) {
      const node = nodes[i];
      if (node.place === "External") {
        continue;
      }
      const container = findContainer(node.name);
      tempSortedNodes.push({
        ...node,
        id: node.name,
        sn: `${i + 1}.`,
        version: node.version,
        name: `${node.name[0].toUpperCase()}${node.name.substring(1)}`,
        stop: container?.State,
      });
    }
    sortedNodes = [...tempSortedNodes];
  }

  async function stopChildContainers(nodes: string[]) {
    loading = true;
    const result = await stop_child_swarm_containers({
      nodes,
      host: $selectedNode,
    });
    await handle_after_request(result);
  }

  async function startChildContainer(nodes: string[]) {
    loading = true;
    const result = await start_child_swarm_containers({
      nodes,
      host: $selectedNode,
    });
    await handle_after_request(result);
  }

  async function updateContainers(nodes: string[]) {
    loading = true;
    const result = await update_child_swarm_containers({
      nodes,
      host: $selectedNode,
    });
    await handle_after_request(result);
  }

  async function handle_after_request(result) {
    if (!result.success) {
      errorMessage = true;
    }
    message = result.message;
    await setupNodes();
    show_notification = true;
    loading = false;
  }

  async function restartAllContainer() {
    nodes_to_be_modified = [];
    for (let i = 0; i < sortedNodes.length; i++) {
      nodes_to_be_modified.push(`${sortedNodes[i].id}.sphinx`);
    }

    loading = true;

    const stop_result = await stop_child_swarm_containers({
      nodes: nodes_to_be_modified,
      host: $selectedNode,
    });

    const start_result = await start_child_swarm_containers({
      nodes: nodes_to_be_modified,
      host: $selectedNode,
    });

    await setupNodes();

    loading = false;

    message = "Restarted All node successfully";

    if (start_result === false || stop_result === false) {
      errorMessage = true;
      message = start_result.message || stop_result.message;
    }

    if (start_result.success && start_result.data.stack_error) {
      message = `Start Containers: ${start_result.data.stack_error}`;
      errorMessage = true;
    }

    if (stop_result.success && stop_result.data.stack_error) {
      message = `Stop Containers: ${stop_result.data.stack_error}`;
      errorMessage = true;
    }
    show_notification = true;
  }

  async function stopAllContainer() {
    nodes_to_be_modified = [];
    for (let i = 0; i < sortedNodes.length; i++) {
      nodes_to_be_modified.push(`${sortedNodes[i].id}.sphinx`);
    }
    await stopChildContainers(nodes_to_be_modified);
  }

  async function startAllContainer() {
    nodes_to_be_modified = [];
    for (let i = 0; i < sortedNodes.length; i++) {
      nodes_to_be_modified.push(`${sortedNodes[i].id}.sphinx`);
    }
    await startChildContainer(nodes_to_be_modified);
  }

  async function upgradeAllContainer() {
    nodes_to_be_modified = [];
    for (let i = 0; i < sortedNodes.length; i++) {
      nodes_to_be_modified.push(`${sortedNodes[i].id}`);
    }
    await updateContainers(nodes_to_be_modified);
  }

  export let back = () => {};
</script>

<main>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="back" on:click={back}>
    <ArrowLeft size={32} />
  </div>
  <h2 class="node_host">{$selectedNode}</h2>
  {#if show_notification}
    <div class="success_toast_container">
      <ToastNotification
        lowContrast
        kind={errorMessage ? "error" : "success"}
        title={errorMessage ? "Error" : "Success"}
        subtitle={message}
        timeout={3000}
        on:close={(e) => {
          e.preventDefault();
          show_notification = false;
          errorMessage = false;
        }}
        fullWidth={true}
      />
    </div>
  {/if}
  {#if loading}
    <Loading />
  {/if}
  {#if sortedNodes.length > 0}
    <DataTable
      headers={[
        { key: "sn", value: "S/N" },
        { key: "name", value: "Name" },
        // { key: "version", value: "Version" },
        { key: "update", value: "Update" },
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
            <ToolbarMenuItem on:click={() => restartAllContainer()}
              >Restart all</ToolbarMenuItem
            >
            <ToolbarMenuItem on:click={() => stopAllContainer()} hasDivider
              >Stop All</ToolbarMenuItem
            >
            <ToolbarMenuItem on:click={() => startAllContainer()} hasDivider
              >Start All</ToolbarMenuItem
            >
            <ToolbarMenuItem on:click={() => upgradeAllContainer()} hasDivider
              >Upgrade All</ToolbarMenuItem
            >
          </ToolbarMenu>
        </ToolbarContent>
      </Toolbar>
      <svelte:fragment slot="cell" let:row let:cell>
        {#if cell.key === "stop"}
          {#if cell.value === "restarting"}
            <Button disabled={true}>Restarting...</Button>
          {:else if cell.value === "exited"}
            <Button on:click={() => startChildContainer([`${row.id}.sphinx`])}
              >Start</Button
            >
          {:else}
            <Button
              kind={"danger"}
              on:click={() => stopChildContainers([`${row.id}.sphinx`])}
              >Stop</Button
            >
          {/if}
        {:else if cell.key === "update"}
          <Button on:click={() => updateContainers([`${row.id}`])}
            >Update</Button
          >
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
