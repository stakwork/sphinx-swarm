<script lang="ts">
  import { Loading, PasswordInput } from "carbon-components-svelte";
  import { get_bot_token } from "./api/swarm";
  import { onMount } from "svelte";

  let isLoading = true;
  let adminToken = "";

  onMount(async () => {
    const res = await get_bot_token();
    if (res?.success && res?.data) {
      adminToken = res.data;
    }
    isLoading = false;
  });
</script>

<div class="bot-wrapper">
  {#if isLoading}<Loading />{/if}
  <div class="bot-container">
    <PasswordInput labelText="Admin Token" value={adminToken} readonly />
  </div>
</div>

<style>
  .bot-wrapper { font-size: 1rem; padding: 0px 25px; }
  .bot-container { display: flex; flex-direction: column; gap: 1.5rem; max-width: 600px; }
</style>
