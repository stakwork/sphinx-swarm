<script lang="ts">
  import { ArrowLeft } from "carbon-icons-svelte";
  import { selectedNode } from "./store";
  import { onMount } from "svelte";

  onMount(() => {
    // get internal node for this service
  });

  export let back = () => {};
</script>

<main>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="back" on:click={back}>
    <ArrowLeft size={32} />
  </div>
  <h2>{$selectedNode}</h2>
  <div class="data_table"></div>
</main>

<style>
  main {
    padding: 2.5rem;
  }
  .back {
    margin-bottom: 1rem;
  }
</style>
