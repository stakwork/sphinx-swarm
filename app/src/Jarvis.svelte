<script lang="ts">
  import {
    Button,
    Loading,
    Select,
    SelectItem,
  } from "carbon-components-svelte";
  import { selectedNode, stack } from "./store";
  import { update_node } from "./api/swarm";
  import { getImageVersion, handleGetImageTags } from "./helpers/swarm";
  import { onMount } from "svelte";

  export let updateBody = () => {};
  let selected_tag = "";
  let tags = [];
  let isLoading = true;
  let success = false;
  let notification_message = "";
  let show_notification = false;

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

  onMount(async () => {
    tags = await handleGetImageTags($selectedNode.name);
    isLoading = false;
  });
</script>

<div class="nav-wrapper">
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

  .update_container {
    display: flex;
    gap: 1rem;
    flex-direction: column;
    margin-top: 2rem;
  }
</style>
