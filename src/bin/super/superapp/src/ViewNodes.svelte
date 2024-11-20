<script lang="ts">
  import { ArrowLeft, UpdateNow, Stop } from "carbon-icons-svelte";
  import { selectedNode, remotes } from "./store";
  import { onMount } from "svelte";
  import {
    get_child_swarm_config,
    get_child_swarm_containers,
    restart_child_swarm_containers,
    start_child_swarm_containers,
    stop_child_swarm_containers,
    update_child_swarm_containers,
    get_aws_instance_types,
    update_aws_instance_type,
    get_swarm_instance_type,
    get_child_swarm_image_versions,
    update_swarm_details,
    change_child_swarm_password,
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
    ToolbarBatchActions,
    Modal,
    InlineNotification,
    Select,
    SelectItem,
    TextInput,
  } from "carbon-components-svelte";
  import type { Remote } from "./types/types";

  let loading = true;
  let selectedRowIds = [];
  let nodes = [];
  let containers = [];
  let message = "";
  let errorMessage = false;
  $: sortedNodes = [];
  $: nodes_to_be_modified = [];
  let show_notification = false;
  let aws_instance_types = [];
  let current_instance_type = "";
  let node: Remote | null = null;
  let open_edit_swarm = false;
  let isSubmitting = false;
  let error_notification = false;
  let selected_instance = "";
  let swarm_description = "letnbooks";
  let current_description = "";
  let open_change_swarm_password = false;
  let new_password = "";
  let current_password = "";

  async function setupNodes() {
    const result = await get_child_swarm_config({ host: $selectedNode });
    if (result.success && result.data && result.data.stack_error) {
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

    if (
      swarm_containers.success &&
      swarm_containers.data &&
      swarm_containers.data.stack_error
    ) {
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

  async function getAwsInstanceType() {
    try {
      const instanceTypes = await get_aws_instance_types();
      if (instanceTypes.success) {
        aws_instance_types = [...instanceTypes.data];
      }
    } catch (error) {
      console.log("Error getting AWS Instance Type: ", error);
    }
  }

  async function get_current_service_details() {
    for (let i = 0; i < $remotes.length; i++) {
      const remote = $remotes[i];
      if (remote.host === $selectedNode) {
        node = { ...remote };
        swarm_description = remote.note;
        current_description = remote.note;
        try {
          const response = await get_swarm_instance_type({
            instance_id: remote.ec2_instance_id,
          });
          if (response.success) {
            current_instance_type = response.data.instance_type;
          }
        } catch (error) {
          console.log("ERORR GETTING SWARM INSTANCE TYPE: ", error);
        }
        return;
      }
    }
  }

  function handleOpenEditSwarm() {
    selected_instance = current_instance_type;
    swarm_description = current_description;
    open_edit_swarm = true;
  }

  async function handleEditSwarm() {
    isSubmitting = true;
    try {
      if (selected_instance !== current_instance_type) {
        if (!node || !node.ec2_instance_id) {
          error_notification = true;
          message = "Can't edit this instance type currently";
          isSubmitting = false;
          return;
        }
        const result = await update_aws_instance_type({
          instance_id: node.ec2_instance_id,
          instance_type: selected_instance,
        });
        message = result.message;
        if (result.success) {
          errorMessage = false;
          show_notification = true;
          // close modal
          current_instance_type = selected_instance;
        } else {
          error_notification = true;
          return;
        }
      }

      // update basic swarm details
      const data = {
        id: $selectedNode,
        host: $selectedNode, // to be changed when we are abltto update host
        instance: current_instance_type,
        description: swarm_description,
      };
      let response = await update_swarm_details(data);
      message = response?.message;
      if (response?.success === true) {
        //clear host, instance, description
        isSubmitting = false;

        //close modal
        open_edit_swarm = false;

        //add notification for success
        show_notification = true;
      } else {
        error_notification = true;
      }
    } catch (error) {
      console.log(
        "ERROR EDITING INSTANCE TYPE OR INSTANCE DETAILS: ",
        JSON.stringify(error)
      );
    } finally {
      isSubmitting = false;
    }
  }

  function handleOnCloseEditSwarm() {
    selected_instance = current_instance_type;
    swarm_description = current_description;
    isSubmitting = false;
    open_edit_swarm = false;
  }

  onMount(async () => {
    // get internal node for this service
    await setupNodes();

    await getAwsInstanceType();

    await get_current_service_details();

    await get_image_versions();
  });

  function findContainer(node_name: string) {
    for (let i = 0; i < containers.length; i++) {
      const container = containers[i];
      if (container.Names.includes(`/${node_name}.sphinx`)) {
        return container;
      }
    }
  }

  async function get_image_versions() {
    try {
      const response = await get_child_swarm_image_versions({
        host: $selectedNode,
      });
      if (response.success === true) {
        const version_object = {};
        for (let i = 0; i < response.data.data.length; i++) {
          version_object[response.data.data[i].name] =
            response.data.data[i].version;
        }

        let tempSortedNodes = [];

        for (let i = 0; i < sortedNodes.length; i++) {
          const node = sortedNodes[i];

          tempSortedNodes.push({
            ...node,
            ...(node.version === "latest" && {
              version: version_object[node.name.toLowerCase()],
            }),
          });
        }

        sortedNodes = [...tempSortedNodes];
      }
    } catch (error) {
      console.log(error);
      console.log(
        `Error getting ${$selectedNode} image version: ${JSON.stringify}`
      );
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
      nodes_to_be_modified.push(`${sortedNodes[i].id}`);
    }

    await restart_all_node_handler(nodes_to_be_modified);
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

  async function updateSelectedNodes() {
    let formatted_node_ids = [];
    for (let i = 0; i < selectedRowIds.length; i++) {
      formatted_node_ids.push(`${selectedRowIds[i]}`);
    }
    await updateContainers(formatted_node_ids);
    selectedRowIds = [];
  }

  async function stopSlectedNodes() {
    let formatted_node_ids = [];
    for (let i = 0; i < selectedRowIds.length; i++) {
      formatted_node_ids.push(`${selectedRowIds[i]}.sphinx`);
    }

    await stopChildContainers(formatted_node_ids);
    selectedRowIds = [];
  }

  async function startSelectedNodes() {
    let formatted_node_ids = [];
    for (let i = 0; i < selectedRowIds.length; i++) {
      formatted_node_ids.push(`${selectedRowIds[i]}.sphinx`);
    }

    await startChildContainer(formatted_node_ids);
    selectedRowIds = [];
  }

  async function restartSelectedNodes() {
    let formatted_node_ids = [];

    for (let i = 0; i < selectedRowIds.length; i++) {
      formatted_node_ids.push(`${selectedRowIds[i]}`);
    }

    await restart_all_node_handler(formatted_node_ids);
    selectedRowIds = [];
  }

  async function restart_all_node_handler(nodes: string[]) {
    loading = true;

    const restart_result = await restart_child_swarm_containers({
      nodes,
      host: $selectedNode,
    });

    await setupNodes();

    loading = false;

    message = "Restarted All node successfully";

    if (restart_result === false) {
      errorMessage = true;
      message = restart_result.message;
    }

    if (
      restart_result.success &&
      restart_result.data &&
      restart_result.data.stack_error
    ) {
      message = `Start Containers: ${restart_result.data.stack_error}`;
      errorMessage = true;
    }

    show_notification = true;
  }

  function handleOpenChangeSwarmPassword() {
    open_change_swarm_password = true;
  }

  function handleOnCloseChangePassword() {
    open_change_swarm_password = false;
    current_password = "";
    new_password = "";
  }

  async function handleChangePasword() {
    isSubmitting = true;
    try {
      if (current_password === new_password) {
        error_notification = true;
        message = "current password and new password cannot be the same";
        return;
      }
      const response = await change_child_swarm_password({
        old_password: current_password,
        new_password,
        host: $selectedNode,
      });
      console.log(response);
    } catch (error) {
      console.log("ERROR CHANGING SWARM PASSWORD");
    } finally {
      isSubmitting = false;
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
        { key: "version", value: "Version" },
        { key: "update", value: "Update" },
        { key: "stop", value: "Stop/Start" },
        { key: "restart", value: "Restart" },
      ]}
      selectable
      bind:selectedRowIds
      rows={sortedNodes}
      zebra
    >
      <Toolbar>
        <ToolbarBatchActions>
          <Button
            on:click={() => updateSelectedNodes()}
            kind={"secondary"}
            icon={UpdateNow}>Update</Button
          >
          <Button on:click={() => restartSelectedNodes()}>Restart</Button>
          <Button
            on:click={() => stopSlectedNodes()}
            kind={"danger"}
            icon={Stop}>Stop</Button
          >
          <Button on:click={() => startSelectedNodes()}>Start</Button>
        </ToolbarBatchActions>
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
            <ToolbarMenuItem on:click={() => handleOpenEditSwarm()} hasDivider
              >Edit Swarm</ToolbarMenuItem
            >
            <ToolbarMenuItem
              on:click={() => handleOpenChangeSwarmPassword()}
              hasDivider>Change Password</ToolbarMenuItem
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
        {:else if cell.key === "restart"}
          <Button on:click={() => restart_all_node_handler([`${row.id}`])}
            >Restart</Button
          >
        {:else}
          {cell.value}
        {/if}
      </svelte:fragment>
    </DataTable>

    <Modal
      bind:open={open_edit_swarm}
      modalHeading="Update Swarm"
      primaryButtonDisabled={(current_description === swarm_description &&
        selected_instance === current_instance_type &&
        !selected_instance) ||
        isSubmitting}
      primaryButtonText={isSubmitting ? "Loading..." : "Update"}
      secondaryButtonText="Cancel"
      on:click:button--secondary={() => (open_edit_swarm = false)}
      on:open
      on:close={handleOnCloseEditSwarm}
      on:submit={handleEditSwarm}
    >
      {#if error_notification}
        <InlineNotification
          kind="error"
          title="Error:"
          subtitle={message}
          timeout={3000}
          on:close={(e) => {
            e.preventDefault();
            error_notification = false;
          }}
        />
      {/if}
      <div class="input_container">
        <TextInput
          value={$selectedNode}
          labelText="Host"
          placeholder="Please enter Swarm host..."
          readonly
        />
      </div>
      <div class="input_container">
        <TextInput
          labelText="Description"
          placeholder="Enter Swarm description..."
          bind:value={swarm_description}
        />
      </div>
      <div class="select_instance_container">
        <Select
          on:change={(e) => (selected_instance = e.target.value)}
          helperText="Select Ec2 Instance Size"
          labelText="Ec2 Instance Size"
          selected={selected_instance}
        >
          <SelectItem value={""} text={"Select Size"} />
          {#each aws_instance_types as option}
            <SelectItem value={option.value} text={option.name} />
          {/each}
        </Select>
      </div>
    </Modal>
    <Modal
      bind:open={open_change_swarm_password}
      modalHeading="Update Swarm"
      primaryButtonDisabled={!current_password || !new_password || isSubmitting}
      primaryButtonText={isSubmitting ? "Loading..." : "Update"}
      secondaryButtonText="Cancel"
      on:click:button--secondary={() => (open_change_swarm_password = false)}
      on:open
      on:close={handleOnCloseChangePassword}
      on:submit={handleChangePasword}
    >
      {#if error_notification}
        <InlineNotification
          kind="error"
          title="Error:"
          subtitle={message}
          timeout={8000}
          on:close={(e) => {
            e.preventDefault();
            error_notification = false;
          }}
        />
      {/if}
      <div class="input_container">
        <TextInput
          labelText="Current Password"
          placeholder="Enter Current Password..."
          bind:value={current_password}
        />
      </div>
      <div class="input_container">
        <TextInput
          labelText="New Password"
          placeholder="Enter New Password..."
          bind:value={new_password}
        />
      </div>
    </Modal>
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
    display: flex;
    margin-bottom: 1rem;
    cursor: pointer;
    max-width: fit-content;
  }

  .node_host {
    margin-bottom: 1rem;
  }

  .input_container {
    margin-bottom: 1rem;
  }
</style>
