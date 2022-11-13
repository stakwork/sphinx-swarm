<script lang="ts">
  export let ctrls = [];
  import Ctrl from "./Ctrl.svelte";
</script>

<div class="controls">
  {#each ctrls as ctrl}
    <Ctrl {...ctrl} />
    <div class="spacer" />
  {/each}
</div>

<style>
  .spacer {
    margin-bottom: 1rem;
  }
  .controls {
    padding: 1.5rem;
  }
</style>
