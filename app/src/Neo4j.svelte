<script lang="ts">
  import { Loading, PasswordInput, NumberInput, Button } from "carbon-components-svelte";
  import { get_neo4j_password, update_neo4j_config } from "./api/swarm";
  import { onMount } from "svelte";

  let isLoading = true;
  let neo4jPassword = "Loading Neo4j Password";
  let heap_initial_gb: number = 6;
  let heap_max_gb: number = 6;
  let pagecache_gb: number = 8;
  let tx_total_gb: number = 4;
  let tx_max_gb: number = 1;
  let checkpoint_iops: number = 500;
  let isSaving = false;
  let saveMessage = "";

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

  async function handleUpdateNeo4jConfig() {
    isSaving = true;
    saveMessage = "";
    try {
      const res = await update_neo4j_config({
        heap_initial_gb,
        heap_max_gb,
        pagecache_gb,
        tx_total_gb,
        tx_max_gb,
        checkpoint_iops,
      });
      if (res && res.success) {
        saveMessage = res.message || "Neo4j config updated. Restart neo4j container to apply changes.";
      } else {
        console.log("Error updating neo4j config", res);
        saveMessage = res?.message || "Error updating neo4j config";
      }
    } catch (e) {
      console.error("Error updating neo4j config", e);
      saveMessage = "Error updating neo4j config";
    } finally {
      isSaving = false;
    }
  }

  onMount(async () => {
    // get neo4j password
    await handleGetNeo4jPassword();
    isLoading = false;
  });
</script>

<div class="nav-wrapper">
  {#if isLoading}
    <Loading />
  {/if}

  <div class="neo4j_container">
    <PasswordInput labelText="Password" value={neo4jPassword} readonly />

    <div class="settings">
      <h3>Neo4j Memory & IO Settings</h3>
      <div class="grid">
        <NumberInput
          id="heap_initial_gb"
          label="Heap initial size (GB)"
          min={1}
          bind:value={heap_initial_gb}
        />
        <NumberInput
          id="heap_max_gb"
          label="Heap max size (GB)"
          min={1}
          bind:value={heap_max_gb}
        />
        <NumberInput
          id="pagecache_gb"
          label="Page cache size (GB)"
          min={1}
          bind:value={pagecache_gb}
        />
        <NumberInput
          id="tx_total_gb"
          label="Tx total memory max (GB)"
          min={1}
          bind:value={tx_total_gb}
        />
        <NumberInput
          id="tx_max_gb"
          label="Tx memory max (GB)"
          min={1}
          bind:value={tx_max_gb}
        />
        <NumberInput
          id="checkpoint_iops"
          label="Checkpoint IOPS limit"
          min={1}
          bind:value={checkpoint_iops}
        />
      </div>

      <Button kind="primary" disabled={isSaving} on:click={handleUpdateNeo4jConfig}>
        {#if isSaving}
          Saving...
        {:else}
          Save Neo4j Config
        {/if}
      </Button>
      {#if saveMessage}
        <p class="save-message">{saveMessage}</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .nav-wrapper {
    font-size: 1rem;
    padding: 0px 25px;
  }
  .neo4j_container {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    max-width: 600px;
  }
  .settings .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }
  .save-message {
    margin-top: 0.5rem;
  }
</style>
