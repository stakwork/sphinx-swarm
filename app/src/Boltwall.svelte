<script>
  import { onMount } from "svelte";
  import {
    Select,
    SelectItem,
    Button,
    Loading,
    InlineNotification,
    NumberInput,
  } from "carbon-components-svelte";
  import { getImageVersion, handleGetImageTags } from "./helpers/swarm";
  import { selectedNode, stack } from "./store";
  import {
    get_boltwall_request_per_seconds,
    update_boltwall_request_per_seconds,
    update_node,
  } from "./api/swarm";
  import { XAxis } from "carbon-icons-svelte";
  export let host = "";
  export let updateBody = () => {};
  let link = host ? `https://${host}` : "http://localhost:8444";
  let tags = [];
  let selected_tag = "";
  let isLoading = true;
  let show_notification = false;
  let notification_message = "";
  let success = false;
  let requestPerSeconds = 0;
  let storedRequestPerSeconds = 0;

  onMount(async () => {
    await handleGetRequestPerSeconds();
    tags = await handleGetImageTags($selectedNode.name);
    isLoading = false;
  });

  async function handleGetRequestPerSeconds() {
    try {
      const rps = await get_boltwall_request_per_seconds();
      if (rps && rps.success) {
        requestPerSeconds = rps.data;
        storedRequestPerSeconds = rps.data;
      }
    } catch (error) {
      console.log("Error getting boltwall request per seconds: ", error);
    }
  }

  async function handleUpdateNodeVersion() {
    isLoading = true;
    try {
      const res = await update_node($selectedNode.name, selected_tag);
      if (res === "{}") {
        await getImageVersion(stack, selectedNode);
        updateBody();
        success = true;
        notification_message = `${$selectedNode.name} version updated successfully`;
      }
    } catch (error) {
      console.log(error);
    } finally {
      isLoading = false;
      show_notification = true;
    }
  }

  async function handleUpdateBoltwallRequestPerSeconds() {
    isLoading = true;
    try {
      const res = await update_boltwall_request_per_seconds({
        request_per_seconds: Number(requestPerSeconds),
      });
      notification_message = res?.message;
      if (res.success) {
        success = true;
        storedRequestPerSeconds = Number(requestPerSeconds);
      }
    } catch (error) {
      console.log(error);
    } finally {
      isLoading = false;
      show_notification = true;
    }
  }
</script>

<div class="nav-wrapper">
  {#if show_notification}
    <InlineNotification
      lowContrast
      kind={success ? "success" : "error"}
      title={success ? "Success:" : "Error:"}
      subtitle={notification_message}
      timeout={9000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  <div class="title">Boltwall URL:</div>
  <div class="spacer" />
  <div>{link}</div>
  {#if isLoading}
    <Loading />
  {/if}

  <div class="update_container">
    <Select
      labelText={`Update ${$selectedNode.name} version`}
      selected="g10"
      on:change={(e) => (selected_tag = e.target.value)}
    >
      {#each tags as tag}
        <SelectItem value={`${tag}`} text={`${tag}`} />
      {/each}
    </Select>
    <Button
      on:click={handleUpdateNodeVersion}
      disabled={!selected_tag || $selectedNode.version === selected_tag}
      >Update Version</Button
    >
  </div>
  <div class="updateReqPerSecs">
    <NumberInput
      label="Update Request Per seconds"
      bind:value={requestPerSeconds}
    />
    <Button
      on:click={handleUpdateBoltwallRequestPerSeconds}
      disabled={!requestPerSeconds ||
        storedRequestPerSeconds === requestPerSeconds}
      >Update Request Per Seconds</Button
    >
  </div>
</div>

<style>
  .nav-wrapper {
    font-size: 1rem;
    padding: 0px 25px;
  }
  .title {
    margin-top: 1rem;
  }
  .spacer {
    height: 1rem;
  }

  .update_container {
    display: flex;
    gap: 1rem;
    flex-direction: column;
    margin-top: 2rem;
  }

  .updateReqPerSecs {
    margin-top: 2rem;
  }
</style>
