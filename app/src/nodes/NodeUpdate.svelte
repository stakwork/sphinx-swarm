<script lang="ts">
  import { Button, InlineLoading } from "carbon-components-svelte";
  import Upgrade from "carbon-icons-svelte/lib/Upgrade.svelte";
  import * as api from "../api";
  import { selectedNode, node_state, stack } from "../store";
  import { getImageVersion } from "../helpers/swarm";

  let updating = false;

  async function upgradeContainer() {
    let name = $selectedNode.name;
    if (!name) return;
    updating = true;
    await api.swarm.update_node(name);
    await getImageVersion(stack, selectedNode);
    updating = false;
  }
</script>

<aside class="node-action-wrap">
  {#if updating}
    <InlineLoading description={`Updating ${$selectedNode.name}...`} />
  {:else}
    <Button
      kind="primary"
      disabled={updating}
      class="btn-stop"
      on:click={upgradeContainer}
      iconDescription={`Upgrade ${$selectedNode.name}`}
      icon={Upgrade}
      size="field"
    />
  {/if}
</aside>

<style>
  .node-action-wrap {
    margin-left: 1rem;
  }
</style>
