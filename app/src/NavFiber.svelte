<script>
  import { Button, TextInput, InlineLoading } from "carbon-components-svelte";
  import { add_boltwall_admin_pubkey } from "./api/swarm";

  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8001";
  $: pubkey = "";
  $: loading = false;

  async function setSuperAdmin() {
    loading = true;
    console.log(pubkey);
    const result = await add_boltwall_admin_pubkey(pubkey);
    console.log(result);
    loading = false;
  }
</script>

<div class="nav-wrapper">
  <Button target="_blank" href={link}>Open Second Brain</Button>
  <div class="super-admin-container">
    <TextInput
      labelText="Super Admin Pubkey"
      placeholder="Enter super admin pubkey..."
      bind:value={pubkey}
    />
    <div class="set-super-admin-btn-container">
      <Button on:click={setSuperAdmin} disabled={!pubkey || loading}>
        {#if loading}
          <InlineLoading />
        {:else}
          Set Super Admin
        {/if}
      </Button>
    </div>
  </div>
</div>

<style>
  .nav-wrapper {
    padding: 0px 25px;
  }
  .super-admin-container {
    display: flex;
    flex-direction: column;
    margin-top: 1.5rem;
  }

  .set-super-admin-btn-container {
    margin-top: 0.5rem;
  }
</style>
