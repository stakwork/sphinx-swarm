<script>
  import { Toggle } from "carbon-components-svelte";

  export let id;
  export let description = "";
  export let toggled = false;

  $: disabled = false;
</script>

<div class="endpoint-container">
  <p>{description}</p>
  <div class="toggle-container">
    <Toggle
      labelA=""
      labelB=""
      bind:toggled
      on:toggle={(e) => {
        console.log(e.detail);
        disabled = true;
      }}
      {disabled}
    />
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
