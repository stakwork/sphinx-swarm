<script lang="ts">
  import Login from "./auth/Login.svelte";
  import Dashboard from "./Dashboard.svelte";
  import { activeUser } from "./store";
</script>

<main>
  {#if !$activeUser}
    <Login />
  {:else}
    <Dashboard />
  {/if}
</main>

<style>
</style>
