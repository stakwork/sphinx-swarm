<script lang="ts">
  export let ctrls = [];
  import Ctrl from "./Ctrl.svelte";
</script>

<div class="controls">
  {#each ctrls as ctrl}
    {#if ctrl.type === "dropdown"}
      <Ctrl {...ctrl} selectedId={ctrl.items[0].id} />
      <div class="&& spacer" />
    {:else}
      <Ctrl {...ctrl} />
      <div class="spacer" />
    {/if}
  {/each}
</div>

<style>
</style>
