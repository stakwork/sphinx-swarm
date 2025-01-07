<script lang="ts">
  import {
    Button,
    Loading,
    Select,
    SelectItem,
    TextInput,
    PasswordInput,
  } from "carbon-components-svelte";
  import { selectedNode, stack } from "./store";
  import { update_node, get_neo4j_password } from "./api/swarm";
  import { getImageVersion, handleGetImageTags } from "./helpers/swarm";
  import { onMount } from "svelte";

  export let updateBody = () => {};
  let selected_tag = "";
  let tags = [];
  let isLoading = true;
  let success = false;
  let notification_message = "";
  let show_notification = false;
  let neo4jPassword = "Loading Neo4j Password";

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

  async function handleGetNeo4jPassword() {
    try {
      const res = await get_neo4j_password();
      if (res.success === true) {
        neo4jPassword = res.data;
        return;
      }
      console.log("There was an error getting neo4j password", res);
    } catch (error) {
      console.error("Error getting neo4j password", error);
    }
  }

  onMount(async () => {
    // get neo4j password
    await handleGetNeo4jPassword();
    tags = await handleGetImageTags($selectedNode.name);
    isLoading = false;
  });
</script>

<div class="nav-wrapper">
  {#if isLoading}
    <Loading />
  {/if}

  <div class="neo4j_container">
    <PasswordInput labelText="Neo4j Password" value={neo4jPassword} readonly />
  </div>

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
