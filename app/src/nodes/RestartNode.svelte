<script lang="ts">
  import { Button, InlineLoading } from "carbon-components-svelte";
  import { selectedNode } from "../store";
  import { Upgrade, Restart } from "carbon-icons-svelte";

  function restartContainer() {}
</script>

<aside>
  {#if updating}
    <InlineLoading description={`Restarting ${$selectedNode.name}...`} />
  {:else}
    <Button
      kind="primary"
      disabled={updating}
      class="btn-stop"
      on:click={restartContainer}
      iconDescription={`Upgrade ${$selectedNode.name}`}
      icon={Restart}
      size="field"
    />
  {/if}
</aside>

<style></style>
