<script>
  import { Toggle, InlineNotification } from "carbon-components-svelte";
  import { update_paid_endpoint } from "../api/swarm";

  export let id;
  export let description = "";
  export let toggled = false;

  $: disabled = false;
  $: show_notification = false;
  $: message = "";
  $: success = false;

  async function handleUpdatePaidEndpoint(status) {
    disabled = true;
    const result = await update_paid_endpoint(id, status);
    const parsedResult = JSON.parse(result);
    message = parsedResult.message;
    success = parsedResult.success;
    show_notification = true;
    disabled = false;
  }
</script>

<div class="container">
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
  <div class="endpoint-container">
    <p>{description}</p>
    <div class="toggle-container">
      <Toggle
        labelA=""
        labelB=""
        bind:toggled
        on:toggle={(e) => {
          handleUpdatePaidEndpoint(e.detail.toggled);
        }}
        {disabled}
      />
    </div>
  </div>
</div>

<style>
  .endpoint-container {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .toggle-container {
    display: flex;
    justify-content: center;
    align-items: center;
    margin-bottom: 1rem;
  }
</style>
