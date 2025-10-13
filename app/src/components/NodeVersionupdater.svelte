<script lang="ts">
  import {
    Button,
    Loading,
    Select,
    SelectItem,
  } from "carbon-components-svelte";
  import { getImageVersion, handleGetImageTags } from "../helpers/swarm";
  import { selectedNode, stack } from "../store";
  import { onMount } from "svelte";
  import { update_node } from "../api/swarm";

  let tags = [];
  let isLoading = true;
  let selected_tag = "";
  export let handleSuccess = () => {};
  export let handleError = (errMsg: string) => {};

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
      }
      isLoading = false;
      handleSuccess();
    } catch (error) {
      isLoading = false;
      const errMsg = error instanceof Error ? error.message : String(error);
      handleError(errMsg);
    }
  }
</script>

<main>
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
</main>

<style>
  .update_container {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
</style>
