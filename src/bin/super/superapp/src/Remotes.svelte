<script lang="ts">
  import {
    Button,
    DataTable,
    Toolbar,
    ToolbarContent,
    ToolbarSearch,
    Modal,
    ToastNotification,
    InlineNotification,
    Loading,
    ToolbarBatchActions,
    Select,
    SelectItem,
    Checkbox,
  } from "carbon-components-svelte";
  import { UpdateNow, Stop } from "carbon-icons-svelte";

  import Healthcheck from "./Healthcheck.svelte";
  import UploadIcon from "carbon-icons-svelte/lib/Upload.svelte";
  import Tribes from "./Tribes.svelte";
  import * as api from "../../../../../app/src/api";
  import { remotes } from "./store";
  import { onMount } from "svelte";
  import type { Remote } from "./types/types";
  import { getSwarmNumber, isValidVanityAddress } from "./utils/index";
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

  let open_create_ec2 = false;
  let show_notification = false;
  let message = "";
  let isSubmitting = false;
  let error_notification = false;
  let errorMessage = false;
  let loading = true;
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
  let vanity_address_error = "";
  let repo_2_graph_checked = false;
  let repo_2_graph_env = {
    JARVIS_FEATURE_FLAG_WFA_SCHEMAS: "true",
    JARVIS_FEATURE_FLAG_CODEGRAPH_SCHEMAS: "true",
  };

  let selectedRowIds = [];

  export let viewNode = () => {};

  async function getConfig() {
    const conf = await api.swarm.get_config();
    if (conf && conf.stacks && conf.stacks.length) {
      remotes.set(conf.stacks);
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
          if (!/swarm\d+/.test(host)) {
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

    loading = false;

    await getConfigSortByUnhealthy();
  });

  function remoterow(r: Remote) {
    let swarmNumber = "";
    if (r.default_host) {
      swarmNumber = getSwarmNumber(r.default_host);
    }
    return { ...r, id: r.host, number: swarmNumber };
  }

  function handleViewNodes(id: string) {
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
    let formattedAddress = "";

    if (vanity_address) {
      formattedAddress = `${vanity_address}${domain}`;
    }

    try {
      const data = {
        name: `${name}${swarm_name_suffix}`,
        vanity_address: formattedAddress,
        instance_type: selected_instance,
        ...(repo_2_graph_checked && { env: { ...repo_2_graph_env } }),
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
        repo_2_graph_checked = false;

        await getConfig();

        await getConfigSortByUnhealthy();
      } else {
        error_notification = true;
      }
    } catch (error) {
      console.log("Error creating ec2 instance: ", error);
    }
    isSubmitting = false;
  }

  function updateVanityAddressWidth(event) {
    vanity_address_error = "";
    vanity_address = event.target.value.replace(/\s+/g, "");
    vanity_address_error = isValidVanityAddress(vanity_address);

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
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <DataTable
    headers={[
      { key: "host", value: "Host" },
      { key: "number", value: "Number" },
      { key: "note", value: "Description" },
      { key: "ec2", value: "Instance" },
      { key: "public_ip_address", value: "Public IP" },
      { key: "private_ip_address", value: "Private IP" },
      { key: "health", value: "Health" },
    ]}
    rows={$remotes.map(remoterow)}
    selectable
    bind:selectedRowIds
    batchSelection
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
      {:else if cell.key === "host"}
        <p class="host_name" on:click={() => handleViewNodes(row.id)}>
          {cell.value}
        </p>
      {:else if cell.key === "tribes"}
        <Tribes host={row.id} />
      {:else}
        {cell.value}
      {/if}
    </svelte:fragment>
  </DataTable>
  <Modal
    bind:open={open_create_ec2}
    modalHeading="Create New Swarm Ec2 Instance"
    primaryButtonDisabled={isSubmitting ||
      !name ||
      !selected_instance ||
      vanity_address_error.length > 0}
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
      <small class="error-message">{vanity_address_error}</small>
    </div>
    <div class="checkbox_container">
      <Checkbox labelText="Repo2Graph" bind:checked={repo_2_graph_checked} />
    </div>
  </Modal>
</main>

<style>
  main {
    overflow: auto;
    max-height: var(--body-height);
    width: 100%;
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

  .error-message {
    color: #d32f2f;
    margin-top: 0;
    font-size: 0.7rem;
  }

  .select_instance_container {
    margin-bottom: 1rem;
    margin-top: 1rem;
  }

  .host_name {
    text-decoration: underline;
    cursor: pointer;
  }

  .checkbox_container {
    margin-top: 1rem;
  }
</style>
