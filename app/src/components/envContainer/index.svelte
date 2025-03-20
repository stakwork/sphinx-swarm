<script lang="ts">
  import EnvRow from "./envRow.svelte";

  interface EnvArray {
    key: string;
    value: string;
  }
  export let EnvArray: EnvArray[];
</script>

<div class="container">
  {#each EnvArray as { key, value }}
    <EnvRow {key} {value} />
  {/each}
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding-top: 1rem;
    padding-bottom: 1rem;
  }
</style>
