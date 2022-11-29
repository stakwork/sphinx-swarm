<script lang="ts">
  import {
    NumberInput,
    Dropdown,
    TextInput,
    Button,
  } from "carbon-components-svelte";

  export let value;
  export let type;
  export let items = [];
  export let name = "";
  export let selectedId = "";
  export let action;
  export let tag = "";

  function runAction() {
    if (action) {
      action(tag);
    }
  }
</script>

{#if type === "number"}
  <NumberInput label={name} bind:value />
{:else if type === "dropdown" && items}
  <Dropdown titleText={name} bind:selectedId {items} />
{:else if type === "text"}
  <TextInput labelText={name} placeholder={name} bind:value />
{:else if type === "button"}
  <Button on:click={runAction}>{name}</Button>
{/if}
