<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Input from "../input/input.svelte";
  export let value;
  export let key;

  const dispatch = createEventDispatcher();

  function handleChange(value) {
    dispatch("update", { key, value });
  }
</script>

<div class="container">
  <div class="keyContainer"><p class="key">{key}</p></div>
  <Input label="" bind:value onInput={handleChange} />
</div>

<style>
  .container {
    display: flex;
    align-items: center;
    gap: 2rem;
    width: 100%;
  }

  .key {
    font-weight: 500;
    font-size: 1rem;
    font-family: "Barlow";
  }

  .keyContainer {
    width: 32rem;
  }
</style>
