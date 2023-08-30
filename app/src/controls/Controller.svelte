<script lang="ts">
  import { selectedNode, node_state, stack } from "../store";
  import Controls from "./Controls.svelte";
  import { controls } from "./controls";
  import RelayControls from "../relay/RelayControls.svelte";
  import TribeControls from "../tribes/TribeControls.svelte";
  import Close from "carbon-icons-svelte/lib/Close.svelte";
  import Lnd from "../lnd/Lnd.svelte";
  import Bitcoin from "../btc/Bitcoin.svelte";
  import Proxy from "../Proxy.svelte";
  import NavFiber from "../NavFiber.svelte";
  import Boltwall from "../Boltwall.svelte";
  import { IS_DEV } from "../api/cmd";
  import { chipSVG } from "../nodes";
  import FirstConnect from "./FirstConnect.svelte";

  $: type = $selectedNode && $selectedNode.type;
  $: ctrls = $selectedNode && controls[type];

  // tag is the name of the container itself
  $: tag = $selectedNode && $selectedNode.name;

  function closeSidebar() {
    selectedNode.set(null);
  }

  function openHsmdUI() {
    if (IS_DEV) {
      window.open("http://localhost:8080", "_blank");
    }
  }

  $: hasHsmd =
    $selectedNode &&
    $selectedNode.plugins &&
    $selectedNode.plugins.includes("HsmdBroker");
</script>

{#if $stack.nodes.length && !$stack.ready}
  <div class="main" style="width: 30rem">
    <FirstConnect />
  </div>
{:else if ctrls}
  <div
    class="main"
    style={`width: ${type === "Lnd" || "Cln" ? "35rem" : "23rem"}`}
  >
    <section class="close-btn-wrap">
      <button on:click={closeSidebar}>
        <Close size={24} />
      </button>
    </section>

    <header>
      <img
        src={`swarm/${type.toLowerCase()}.png`}
        class="node-top-img"
        alt="node "
      />
      {$selectedNode.name}
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      {#if hasHsmd}
        <div class="hsmd-wrap" on:click={openHsmdUI}>{@html chipSVG}</div>
      {/if}
    </header>
    <div class="ctrls">
      {#if type === "Relay"}
        <RelayControls {tag} />
      {:else if type === "Tribes"}
        <TribeControls url={$selectedNode.url} />
      {:else if type === "Lnd"}
        <Lnd {tag} />
      {:else if type === "Btc"}
        <Bitcoin {tag} />
      {:else if type === "Proxy"}
        <Proxy {tag} />
      {:else if type === "NavFiber"}
        <NavFiber host={$selectedNode.host} />
      {:else if type === "BoltWall"}
        <Boltwall host={$selectedNode.host} />
      {:else if type === "Cln"}
        <Lnd {tag} {type} />
      {:else}
        <Controls {ctrls} {tag} />
      {/if}
    </div>
    {#if $node_state === "exited"}
      <div class="overlay" />
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
    border-radius: 0rem;
    position: fixed;
    right: 0rem;
    top: 4.14rem;
    background: #1a242e;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.35);
    animation-name: sidebar;
    animation-duration: 40ms;
  }
  .ctrls {
    position: absolute;
    z-index: 50;
    width: 100%;
    height: 100px;
  }
  .overlay {
    position: absolute;
    z-index: 51;
    background: rgba(0, 0, 0, 0.25);
    width: 100%;
    top: 0;
    bottom: 0;
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

  .close-btn-wrap {
    position: fixed;
    margin-left: -33px;
    margin-top: 23px;
    cursor: pointer;
    width: 30px;
    outline: 0;
    z-index: 10;
  }

  .close-btn-wrap button {
    padding: 5px;
    background: #1a242e;
    color: #fff;
    outline: 0;
    border: 0;
    border-top-left-radius: 10px;
    border-bottom-left-radius: 10px;
    box-shadow: 0 4px 8px 0 #1a242e, 0 6px 20px 0 #1a242e;
    cursor: pointer;
    box-shadow: none;
  }
  .hsmd-wrap {
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-left: 0.85rem;
  }
</style>
