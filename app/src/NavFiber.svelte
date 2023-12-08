<script>
  import {
    Button,
    TextInput,
    InlineLoading,
    InlineNotification,
  } from "carbon-components-svelte";
  import { add_boltwall_admin_pubkey } from "./api/swarm";

  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8001";
  $: pubkey = "";
  $: loading = false;
  $: show_notification = false;
  $: success = false;
  $: message = "";

  async function setSuperAdmin() {
    loading = true;
    const result = await add_boltwall_admin_pubkey(pubkey);
    const parsedResult = JSON.parse(result);
    success = parsedResult.success || false;
    message = parsedResult.message;
    show_notification = true;
    pubkey = "";
    loading = false;
  }
</script>

<div class="nav-wrapper">
  <Button target="_blank" href={link}>Open Second Brain</Button>
  <div class="super-admin-container">
    {#if show_notification}
      <InlineNotification
        lowContrast
        kind={success ? "success" : "error"}
        title={success ? "Success:" : "Error:"}
        subtitle={message}
        timeout={3000}
        on:close={(e) => {
          e.preventDefault();
          show_notification = false;
        }}
      />
    {/if}
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
