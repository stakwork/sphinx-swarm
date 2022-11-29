<script lang="ts">
  export let ctrls = [];
  import Ctrl from "./Ctrl.svelte";

  export let tag = "";
</script>

<div class="controls">
  {#each ctrls as ctrl}
    <Ctrl {...ctrl} {tag} />
    <div class="spacer" />
  {/each}
</div>

<style>
</style>
