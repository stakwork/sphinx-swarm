<script>
  import {
    Button,
    TextInput,
    InlineLoading,
    InlineNotification,
  } from "carbon-components-svelte";
  import {
    add_boltwall_admin_pubkey,
    get_super_admin,
    add_boltwall_sub_admin_pubkey,
  } from "./api/swarm";
  import { onMount } from "svelte";

  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8001";
  $: pubkey = "";
  $: loading = false;
  $: show_notification = false;
  $: success = false;
  $: message = "";
  $: superAdminExist = false;
  $: superAdminPubkey = "";

  async function setSuperAdmin() {
    const result = await add_boltwall_admin_pubkey(pubkey);
    const parsedResult = JSON.parse(result);
    success = parsedResult.success || false;
    message = parsedResult.message;
    superAdminExist = true;
    show_notification = true;
    superAdminPubkey = pubkey;
    pubkey = "";
  }

  async function setSubAdmin() {
    const result = await add_boltwall_sub_admin_pubkey(pubkey);
    const parsedResult = JSON.parse(result);
    success = parsedResult.success || false;
    message = parsedResult.message;
    show_notification = true;
    pubkey = "";
  }

  async function handleSubmit() {
    loading = true;
    if (!superAdminExist) {
      await setSuperAdmin();
    } else {
      await setSubAdmin();
    }
    loading = false;
  }

  async function checkSuperAdminExist() {
    const result = await get_super_admin();
    const parsedResult = JSON.parse(result);
    if (
      parsedResult?.success &&
      parsedResult.message === "super admin record"
    ) {
      superAdminExist = true;
      superAdminPubkey = parsedResult.data.pubkey;
    }
  }

  function toggleAdmin() {
    superAdminExist = !superAdminExist;
  }

  onMount(() => {
    checkSuperAdminExist();
  });
</script>

<div class="nav-wrapper">
  <Button target="_blank" href={link}>Open Second Brain</Button>
  <div class="super-admin-container">
    {#if superAdminPubkey}
      <div class="update_super_admin_container">
        <button class="update_super_admin_btn" on:click={toggleAdmin}>
          {#if !superAdminExist && superAdminPubkey}
            Add Sub Admin
          {:else}
            Update Super Admin pubkey
          {/if}</button
        >
      </div>
    {/if}
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
      labelText={`${
        superAdminExist ? "Sub Admin Pubkey" : "Super Admin Pubkey"
      }`}
      placeholder={`${
        superAdminExist
          ? "Enter sub admin pubkey..."
          : "Enter super admin pubkey..."
      }`}
      bind:value={pubkey}
    />
    <div class="set-super-admin-btn-container">
      <Button on:click={handleSubmit} disabled={!pubkey || loading}>
        {#if loading}
          <InlineLoading />
        {:else}
          Submit
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

  .update_super_admin_container {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 2rem;
  }

  .update_super_admin_btn {
    border: none;
    padding: 0.7rem 1rem;
    border-radius: 0.3rem;
    cursor: pointer;
  }

  .update_super_admin_btn:hover {
    background-color: #f5f5dc;
  }
</style>
