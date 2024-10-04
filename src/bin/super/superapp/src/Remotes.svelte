<script lang="ts">
  import {
    Button,
    DataTable,
    Toolbar,
    ToolbarContent,
    ToolbarSearch,
    Modal,
    TextInput,
    ToastNotification,
    InlineNotification,
    Loading,
    ToolbarBatchActions,
    Select,
    SelectItem,
  } from "carbon-components-svelte";
  import { UpdateNow, Stop } from "carbon-icons-svelte";

  import Healthcheck from "./Healthcheck.svelte";
  import UploadIcon from "carbon-icons-svelte/lib/Upload.svelte";
  import Tribes from "./Tribes.svelte";
  import * as api from "../../../../../app/src/api";
  import { remotes, tribes } from "./store";
  import { onMount } from "svelte";
  import type { Remote } from "./types/types";
  import { getSwarmNumber, splitHost } from "./utils/index";
  import { selectedNode } from "./store";
  import {
    create_new_swarm_ec2,
    get_child_swarm_config,
    start_child_swarm_containers,
    stop_child_swarm_containers,
    update_child_swarm_containers,
    get_aws_instance_types,
    restart_child_swarm_containers,
  } from "../../../../../app/src/api/swarm";

  let open_create_edit = false;
  let open_delete = false;
  let open_create_ec2 = false;
  let host = "";
  let description = "";
  let instance = "";
  let show_notification = false;
  let message = "";
  let isSubmitting = false;
  let error_notification = false;
  let isUpdate = false;
  let swarm_id = "";
  let delete_host = "";
  let errorMessage = false;
  let loading = false;
  let errors = [];
  let name = "";
  let vanity_address = "";
  let domain = ".sphinx.chat";
  let swarm_name_suffix = "-Swarm";
  const max_input_with = 600;
  let vanity_input_width = max_input_with;
  let swarm_name_width = max_input_with;
  let aws_instance_types = [];
  let selected_instance = "";

  let selectedRowIds = [];

  export let viewNode = () => {};

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

  onMount(async () => {
    await getAwsInstanceType();

    await getConfig();

    await getConfigSortByUnhealthy();
  });

  function openAddSwarmModal() {
    open_create_edit = true;
  }

  function handleDeleteSwarm(id: string) {
    open_delete = true;
    delete_host = id;
  }

  async function submitDeleteSwarm() {
    isSubmitting = true;
    try {
      const response = await api.swarm.delete_swarm({
        host: delete_host,
      });
      message = response?.message;
      if (response?.success === "true") {
        await getConfig();
      } else {
        errorMessage = true;
      }
    } catch (error) {
      errorMessage = true;
      message = "An internal Error occurred";
      console.log(`Swarm Delete Error: ${error}`);
    }
    open_delete = false;
    isSubmitting = false;
    show_notification = true;
  }

  function handleOnCloseDelete() {
    delete_host = "";
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
    let swarmNumber = "";
    if (r.default_host) {
      swarmNumber = getSwarmNumber(r.default_host);
    }
    return { ...r, id: r.host, number: swarmNumber };
  }

  async function handleSubmitAddSwarm() {
    if (!host) {
      message = "Host is a required field";
      error_notification = true;
      return;
    }
    isSubmitting = true;

    //send data to backened
    let response;
    if (isUpdate) {
      const data = {
        id: swarm_id,
        host: host,
        instance: instance,
        description: description,
      };
      response = await api.swarm.update_swarm_details(data);
    } else {
      const data = {
        host: host,
        instance: instance,
        description: description,
      };
      response = await api.swarm.add_new_swarm(data);
    }
    message = response?.message;

    if (response?.success === true) {
      //get config again
      await getConfig();

      //clear host, instance, description
      clear_swarm_data();
      isSubmitting = false;

      //close modal
      open_create_edit = false;

      //add notification for success
      show_notification = true;
    } else {
      isSubmitting = false;
      error_notification = true;
    }
  }

  function clear_swarm_data() {
    host = "";
    instance = "";
    description = "";
    isUpdate = false;
    swarm_id = "";
  }

  function findSwarm(id: string) {
    for (let i = 0; i < $remotes.length; i++) {
      const swarm = $remotes[i];
      if (swarm.host === id) {
        return swarm;
      }
    }

    return { host: "", ec2: "", note: "" };
  }

  function handleEditSwarm(id: string) {
    isUpdate = true;
    const swarm = findSwarm(id);
    if (swarm.host) {
      open_create_edit = true;
      host = swarm.host;
      description = swarm.note;
      instance = swarm.ec2;
      swarm_id = id;
    }
  }

  function handleOnClose() {
    clear_swarm_data();
  }

  function handleViewNodes(id: string) {
    console.log(id);
    selectedNode.set(id);
    viewNode();
  }

  async function updateSelectedSwarms() {
    loading = true;
    let errors = [];
    for (let i = 0; i < selectedRowIds.length; i++) {
      const host = selectedRowIds[i];

      try {
        const services = await getServices(host, true);

        if (services.length === 0) {
          errors.push(`${host}: Does not have valid service`);
          break;
        }

        const update_result = await update_child_swarm_containers({
          nodes: services,
          host,
        });

        handle_api_res(host, update_result);
      } catch (error) {
        console.log("Error: ", error);
        errors.push(`${host}: Unexpected Error occured`);
      }
    }

    message = "All services updated successfully";
    handle_after_request(errors);
  }

  async function restartSelectedSwarms() {
    loading = true;
    let errors = [];
    for (let i = 0; i < selectedRowIds.length; i++) {
      const host = selectedRowIds[i];

      try {
        const services = await getServices(host, true);

        if (services.length === 0) {
          errors.push(`${host}: Does not have valid service`);
          break;
        }

        const restart_response = await restart_child_swarm_containers({
          nodes: services,
          host,
        });

        handle_api_res(host, restart_response);
      } catch (error) {
        console.log("Error: ", error);
        errors.push(`${host}: Unexpected Error occured`);
      }
    }

    message = "All services Restarted successfully";
    handle_after_request(errors);
  }

  async function stopSlectedSwarms() {
    loading = true;
    let errors = [];
    for (let i = 0; i < selectedRowIds.length; i++) {
      const host = selectedRowIds[i];

      try {
        const services = await getServices(host, false);

        if (services.length === 0) {
          errors.push(`${host}: Does not have valid service`);
          break;
        }

        const stop_result = await stop_child_swarm_containers({
          nodes: services,
          host,
        });

        handle_api_res(host, stop_result);
      } catch (error) {
        console.log("Error: ", error);
        errors.push(`${host}: Unexpected Error occured`);
      }
    }

    message = "All services Stopped successfully";
    handle_after_request(errors);
  }

  async function startSlectedSwarms() {
    loading = true;
    let errors = [];
    for (let i = 0; i < selectedRowIds.length; i++) {
      const host = selectedRowIds[i];

      try {
        const services = await getServices(host, false);

        if (services.length === 0) {
          errors.push(`${host}: Does not have valid service`);
          break;
        }

        const start_result = await start_child_swarm_containers({
          nodes: services,
          host,
        });

        handle_api_res(host, start_result);
      } catch (error) {
        console.log("Error: ", error);
        errors.push(`${host}: Unexpected Error occured`);
      }
    }

    message = "All services Started successfully";
    handle_after_request(errors);
  }

  async function getServices(
    host: string,
    isUpdateService: boolean
  ): Promise<string[]> {
    const services = [];
    try {
      const services_response = await get_child_swarm_config({ host });
      if (
        services_response.success === true &&
        services_response.data &&
        Array.isArray(services_response.data)
      ) {
        for (let i = 0; i < services_response.data.length; i++) {
          services.push(
            isUpdateService
              ? services_response.data[i].name
              : `${services_response.data[i].name}.sphinx`
          );
        }
      } else {
        errors.push(`${host}: ${services_response.message}`);
      }
    } catch (error) {
      console.log("Error getting services: ", error);
      errors.push(`${host}: error getting services`);
    }
    return services;
  }

  function handle_after_request(errors: string[]) {
    if (errors.length > 0) {
      errorMessage = true;
      message = errors.join(", ");
    }
    loading = false;
    show_notification = true;
    selectedRowIds = [];
  }

  function handle_api_res(
    host: string,
    response: { success: boolean; message: string; data: any }
  ) {
    if (response.success === false) {
      // handle error later
      errors.push(`${host}: ${response.message}`);
    }
  }

  function openCreateSwarmEc2() {
    open_create_ec2 = true;
  }

  function handleOnCloseCreateEc2() {
    open_create_ec2 = false;
    name = "";
    vanity_address = "";
    selected_instance = "";
    vanity_input_width = max_input_with;
    swarm_name_width = max_input_with;
  }

  async function handleSubmitCreateEc2() {
    isSubmitting = true;
    try {
      const data = {
        name: `${name}${swarm_name_suffix}`,
        vanity_address: `${vanity_address}${domain}`,
        instance_type: selected_instance,
      };

      const response = await create_new_swarm_ec2(data);
      message = response.message;
      if (response.success === true) {
        open_create_ec2 = false;
        name = "";
        vanity_address = "";
        selected_instance = "";
        vanity_input_width = max_input_with;
        swarm_name_width = max_input_with;
        show_notification = true;
      } else {
        error_notification = true;
      }
    } catch (error) {
      console.log("Error creating ec2 instance: ", error);
    }
    isSubmitting = false;
  }

  function updateVanityAddressWidth(event) {
    vanity_address = event.target.value.replace(/\s+/g, "");
    const span = document.querySelector(".vanity_address_measure");
    vanity_input_width = span.offsetWidth;
    if (!vanity_input_width) {
      vanity_input_width = max_input_with;
    }
  }

  function updateSwarmnameWidth(event) {
    name = event.target.value.replace(/\s+/g, "");
    const span = document.querySelector(".swarm_name_measure");
    swarm_name_width = span.offsetWidth;
    if (!swarm_name_width) {
      swarm_name_width = max_input_with;
    }
  }
</script>

<main>
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
  <DataTable
    headers={[
      { key: "host", value: "Host" },
      { key: "number", value: "Number" },
      { key: "note", value: "Description" },
      { key: "ec2", value: "Instance" },
      { key: "tribes", value: "Tribes" },
      { key: "health", value: "Health" },
      { key: "nodes", value: "Nodes" },
      { key: "edit", value: "Edit" },
      { key: "delete", value: "Delete" },
    ]}
    rows={$remotes.map(remoterow)}
    selectable
    bind:selectedRowIds
  >
    <Toolbar>
      <ToolbarBatchActions>
        <Button
          on:click={() => updateSelectedSwarms()}
          kind={"secondary"}
          icon={UpdateNow}>Update</Button
        >
        <Button on:click={() => restartSelectedSwarms()}>Restart</Button>
        <Button on:click={() => stopSlectedSwarms()} kind={"danger"} icon={Stop}
          >Stop</Button
        >
        <Button on:click={() => startSlectedSwarms()}>Start</Button>
      </ToolbarBatchActions>
      <ToolbarContent>
        <ToolbarSearch value="" shouldFilterRows />
        <Button kind="tertiary" on:click={openAddSwarmModal} icon={UploadIcon}>
          Add New Swarm
        </Button>
        <Button kind="primary" on:click={openCreateSwarmEc2} icon={UploadIcon}>
          Create New Swarm Ec2 Instance
        </Button>
      </ToolbarContent>
    </Toolbar>
    <svelte:fragment slot="cell" let:row let:cell>
      {#if cell.key === "health"}
        <Healthcheck host={row.id} />
      {:else if cell.key === "tribes"}
        <Tribes host={row.id} />
      {:else if cell.key === "edit"}
        <Button size={"small"} on:click={() => handleEditSwarm(row.id)}>
          Edit
        </Button>
      {:else if cell.key === "nodes"}
        <Button size={"small"} on:click={() => handleViewNodes(row.id)}>
          View Services
        </Button>
      {:else if cell.key === "delete"}
        <Button
          kind="danger"
          size={"small"}
          on:click={() => handleDeleteSwarm(row.id)}
        >
          Delete
        </Button>
      {:else}
        {cell.value}
      {/if}
    </svelte:fragment>
  </DataTable>

  <Modal
    bind:open={open_create_edit}
    modalHeading={isUpdate ? "Update Swarm" : "Add new Swarm"}
    primaryButtonDisabled={isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Confirm"}
    secondaryButtonText="Cancel"
    selectorPrimaryFocus="#db-name"
    on:click:button--secondary={() => (open_create_edit = false)}
    on:open
    on:close={handleOnClose}
    on:submit={handleSubmitAddSwarm}
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
    {#if isUpdate}
      <p>Update Swarm details.</p>
    {:else}
      <p>Add a new swarm to the list of swarms.</p>
    {/if}
    <div class="text_input_container">
      <TextInput
        id="host"
        labelText="Host"
        placeholder="Enter Swarm Host..."
        bind:value={host}
      />
    </div>
    <div class="text_input_container">
      <TextInput
        id="description"
        labelText="Description"
        placeholder="Enter Swarm Description..."
        bind:value={description}
      />
    </div>
    <div class="text_input_container">
      <TextInput
        id="instance"
        labelText="Instance"
        placeholder="Enter Swarm Instance Size..."
        bind:value={instance}
      />
    </div>
  </Modal>
  <Modal
    danger
    bind:open={open_delete}
    modalHeading={`Delete ${delete_host}`}
    primaryButtonDisabled={isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Delete"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open_delete = false)}
    on:open
    on:close={handleOnCloseDelete}
    on:submit={submitDeleteSwarm}
  >
    <p>This is a permanent action and cannot be undone.</p>
  </Modal>
  <Modal
    bind:open={open_create_ec2}
    modalHeading="Create New Swarm Ec2 Instance"
    primaryButtonDisabled={isSubmitting || !name || !selected_instance}
    primaryButtonText={isSubmitting ? "Loading..." : "Create"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open_create_ec2 = false)}
    on:open
    on:close={handleOnCloseCreateEc2}
    on:submit={handleSubmitCreateEc2}
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

    <div class="custom_text_input_container">
      <label class="customlabel" for="label">Swarm Name</label>
      <div class="custom_input_container">
        <div>
          <span class="swarm_name_measure">{name}</span>
          <input
            type="text"
            bind:value={name}
            on:input={updateSwarmnameWidth}
            placeholder="Enter Swarm Name"
            style="width: {swarm_name_width}px;"
            class="custom_input"
          />
        </div>
        {#if name.length > 0}
          <span class="suffix">{swarm_name_suffix}</span>
        {/if}
      </div>
    </div>
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
    <div class="custom_text_input_container">
      <label class="customlabel" for="label">Vanity Address</label>
      <div class="custom_input_container">
        <div>
          <span class={"vanity_address_measure"}>{vanity_address}</span>
          <input
            type="text"
            bind:value={vanity_address}
            on:input={updateVanityAddressWidth}
            placeholder="Enter vanity Address"
            style="width: {vanity_input_width}px;"
            class="custom_input"
          />
        </div>
        {#if vanity_address.length > 0}
          <span class="suffix">{domain}</span>
        {/if}
      </div>
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

  .success_toast_container {
    margin-bottom: 1.2rem;
  }

  .custom_text_input_container {
    margin-top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .custom_input_container {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 1rem;
    overflow: hidden;
    border: solid 1px #494949;
    border-radius: 0.5rem;
    margin-bottom: 1rem;
  }

  .suffix {
    font-size: 1rem;
    font-family: "Barlow";
    width: 100%;
    color: #49c998;
    margin-left: -2px;
  }

  .custom_input {
    border: none;
    outline: none;
    margin: 0;
    font-size: 1rem;
    font-family: "Barlow";
    background-color: transparent;
    width: auto;
    color: white;
    padding: 0;
  }

  .vanity_address_measure {
    visibility: hidden;
    position: absolute;
    white-space: nowrap;
    font-family: "Barlow";
    font-size: 1rem;
    padding: 0;
    border: none;
    margin: 0;
  }

  .swarm_name_measure {
    visibility: hidden;
    position: absolute;
    white-space: nowrap;
    font-family: "Barlow";
    font-size: 1rem;
    padding: 0;
    border: none;
    margin: 0;
  }
</style>
