<script lang="ts">
  import Login from "./auth/Login.svelte";
  import Dashboard from "./Dashboard.svelte";
  import { activeUser, saveUserToStore } from "./store";
</script>

<main>
  {#if !$activeUser}
    <Login {saveUserToStore} />
  {:else}
    <Dashboard />
  {/if}
</main>

<style>
</style>
