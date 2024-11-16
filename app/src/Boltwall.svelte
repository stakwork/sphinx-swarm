<script>
  import { onMount } from "svelte";
  import { Select, SelectItem, Button } from "carbon-components-svelte";
  import { handleGetImageTags } from "./helpers/swarm";
  import { selectedNode } from "./store";
  import { update_node } from "./api/swarm";
  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8444";
  let tags = [];
  let selected_tag = "";

  onMount(async () => {
    tags = await handleGetImageTags($selectedNode.name);
  });

  async function handleUpdateNodeVersion() {
    try {
      const res = await update_node($selectedNode.name, selected_tag);
      console.log(res);
    } catch (error) {
      console.log(error);
    }
  }
</script>

<div class="nav-wrapper">
  <div class="title">Boltwall URL:</div>
  <div class="spacer" />
  <div>{link}</div>

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
