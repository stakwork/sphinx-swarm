<script lang="ts">
  import { selectedNode } from "./store";
  import Controls from "./Controls.svelte";
  import { controls } from "./controls";
  import RelayControls from "./RelayControls.svelte";
  import TribeControls from "./TribeControls.svelte";
  import Lnd from "./Lnd.svelte";

  $: type = $selectedNode && $selectedNode.type;
  $: ctrls = $selectedNode && controls[type];

  // tag is the name of the container itself
  $: tag = $selectedNode && $selectedNode.name;
</script>

{#if ctrls}
  <div class="main">
    <header>
      <img
        src={`swarm/${type.toLowerCase()}.png`}
        class="node-top-img"
        alt="node "
      />
      {$selectedNode.name}
    </header>
    {#if type === "Relay"}
      <RelayControls {tag} />
    {:else if type === "Tribes"}
      <TribeControls url={$selectedNode.url} />
    {:else if type === "Lnd"}
      <Lnd />
    {:else}
      <Controls {ctrls} {tag} />
    {/if}
  </div>
{/if}

<style>
  @keyframes sidebar {
    from {
      transform: translateX(100px);
    }
    to {
      transform: translateX(0px);
    }
  }

  .main {
    font-size: 1.5rem;
    height: calc(100vh - 4.2rem);
    overflow-y: auto;
    width: 23rem;
    transition: width 10s;
    border-radius: 0rem;
    position: fixed;
    right: 0rem;
    top: 4.14rem;
    background: #1a242e;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
    animation-name: sidebar;
    animation-duration: 400ms;
  }
  header {
    font-size: 1rem;
    display: flex;
    align-items: center;
    padding: 1.5rem;
  }
  header .node-top-img {
    width: 1.25rem;
    margin-right: 15px;
  }
</style>
