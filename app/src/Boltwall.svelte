<script>
  import { onMount } from "svelte";
  import {
    Select,
    SelectItem,
    Button,
    Loading,
    InlineNotification,
  } from "carbon-components-svelte";
  import { getImageVersion, handleGetImageTags } from "./helpers/swarm";
  import { selectedNode, stack } from "./store";
  import { update_node } from "./api/swarm";
  export let host = "";
  export let updateBody = () => {};
  let link = host ? `https://${host}` : "http://localhost:8444";
  let tags = [];
  let selected_tag = "";
  let isLoading = true;
  let show_notification = false;
  let notification_message = "";
  let success = false;

  onMount(async () => {
    tags = await handleGetImageTags($selectedNode.name);
    isLoading = false;
  });

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
</style>
