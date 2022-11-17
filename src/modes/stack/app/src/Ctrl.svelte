<script lang="ts">
  import { NumberInput, Dropdown, TextInput } from "carbon-components-svelte";


  export let value;
  export let type;
  export let items = [];
  export let name = "";
  export let selectedId = "";

</script>

{#if type === "number"}
  <NumberInput label={name} bind:value />
{:else if type === "dropdown" && items}
  <Dropdown titleText={name} bind:selectedId {items} />
{:else if type === "text"}
  <TextInput labelText={name} placeholder={name} bind:value />
{/if}
