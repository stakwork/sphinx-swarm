<script>
  import { createEventDispatcher } from "svelte";
  import { Toggle } from "carbon-components-svelte";
  import { update_paid_endpoint } from "../api/swarm";

  export let id;
  export let description = "";
  export let toggled = false;

  const dispatch = createEventDispatcher();

  $: disabled = false;
  $: success = false;

  function sendDataToParent(success) {
    dispatch("customEvent", success);
  }

  async function handleUpdatePaidEndpoint(status) {
    disabled = true;
    const result = await update_paid_endpoint(id, status);
    const parsedResult = JSON.parse(result);
    success = parsedResult.success;
    disabled = false;
    sendDataToParent(success);
  }
</script>

<div class="container">
  <div class="endpoint-container">
    <p class:active={toggled} class="endpoint-description">{description}</p>
    <div class="toggle-container">
      <Toggle
        size="default"
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
    padding: 0.3rem 0;
    border-bottom: 1px solid #00000040;
  }

  .toggle-container {
    display: flex;
    justify-content: center;
    align-items: center;
    margin-bottom: 1rem;
  }

  .endpoint-description {
    font-family: "Barlow";
    font-size: 0.9375rem;
    font-weight: 400;
    line-height: 1.125rem;
    letter-spacing: 0.01em;
    color: #6b7a8d;
  }

  .active {
    color: #ffffff;
  }
</style>
