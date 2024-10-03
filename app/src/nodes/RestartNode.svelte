<script lang="ts">
  import { Button, InlineLoading } from "carbon-components-svelte";
  import { selectedNode, stack } from "../store";
  import { Restart } from "carbon-icons-svelte";
  import * as api from "../api";
  import { getImageVersion } from "../helpers/swarm";
  let restarting = false;

  async function restartContainer() {
    let name = $selectedNode.name;
    if (!name) return;
    console.log("restart!", name);
    restarting = true;
    await api.swarm.restart_node(name);
    await getImageVersion(name, stack, selectedNode);
    restarting = false;
  }
</script>

<aside class="node-action-wrap">
  {#if restarting}
    <InlineLoading description={`Restarting ${$selectedNode.name}...`} />
  {:else}
    <Button
      kind="primary"
      disabled={restarting}
      class="btn-restart"
      on:click={restartContainer}
      iconDescription={`Restart ${$selectedNode.name}`}
      icon={Restart}
      size="field"
    />
  {/if}
</aside>

<style>
  .node-action-wrap {
    margin-left: 1rem;
  }
</style>
