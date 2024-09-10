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
  } from "carbon-components-svelte";
  import Healthcheck from "./Healthcheck.svelte";
  import UploadIcon from "carbon-icons-svelte/lib/Upload.svelte";
  import Tribes from "./Tribes.svelte";
  import * as api from "../../../../../app/src/api";
  import { remotes, tribes } from "./store";
  import { onMount } from "svelte";
  import type { Remote } from "./types/types";
  import { splitHost } from "./utils/index";
  import { selectedNode } from "./store";

  let open_create_edit = false;
  let open_delete = false;
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

  onMount(async () => {
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
    return { ...r, id: r.host };
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
  <DataTable
    headers={[
      { key: "host", value: "Host" },
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
</style>
