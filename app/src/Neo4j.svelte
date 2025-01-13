<script lang="ts">
  import { Loading, PasswordInput } from "carbon-components-svelte";
  import { get_neo4j_password } from "./api/swarm";
  import { onMount } from "svelte";

  let isLoading = true;
  let neo4jPassword = "Loading Neo4j Password";

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
    isLoading = false;
  });
</script>

<div class="nav-wrapper">
  {#if isLoading}
    <Loading />
  {/if}

  <div class="neo4j_container">
    <PasswordInput labelText="Password" value={neo4jPassword} readonly />
  </div>
</div>

<style>
  .nav-wrapper {
    font-size: 1rem;
    padding: 0px 25px;
  }
</style>
