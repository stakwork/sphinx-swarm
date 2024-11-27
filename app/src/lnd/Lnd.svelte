<script>
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import Channels from "./Channels.svelte";
  import Invoices from "./Invoices.svelte";
  import Onchain from "./Onchain.svelte";
  import FirstConnect from "../controls/FirstConnect.svelte";
  import {
    finishedOnboarding,
    isOnboarding,
    selectedNode,
    hsmd,
    hsmdClients,
    lightningPeers,
  } from "../store";
  import { onMount } from "svelte";
  import { get_clients } from "../api/hsmd";
  import { handleGetLightningPeers } from "../helpers/swarm";

  export let tag = "";
  export let type = "";
  $: selected = 0;
  $: $finishedOnboarding, selectCurrentTab();
  function selectCurrentTab() {
    if ($isOnboarding) {
      if (!$finishedOnboarding.hasBalance) {
        selected = 2;
      } else if (
        $finishedOnboarding.hasBalance &&
        !$finishedOnboarding.hasChannels
      ) {
        selected = 0;
      }
    }
  }

  $: peering_url = $selectedNode?.host
    ? `${$selectedNode?.host}:${$selectedNode.peer_port}`
    : `${$selectedNode.name}.sphinx:${$selectedNode.peer_port}`;

  let copied = false;
  function copyAddress() {
    navigator.clipboard.writeText(peering_url);
    copied = true;
    setTimeout(() => (copied = false), 150);
  }

  onMount(async () => {
    try {
      if (type === "Cln") {
        const clients = await get_clients(tag);
        if (clients) hsmdClients.set(clients);
      }

      //get all lightning peers
      await handleGetLightningPeers();
    } catch (error) {
      console.log(error);
    }
  });
</script>

{#if $hsmd}
  <div class="hsmd-wrap">
    <FirstConnect />
  </div>
{:else}
  <div class="lnd-tabs-wrap">
    <div class="node-url">
      <span>Peering Address:</span>
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <span
        on:click={copyAddress}
        style={`transform:scale(${copied ? 1.1 : 1});`}>{peering_url}</span
      >
    </div>
    <Tabs bind:selected>
      <Tab label="Channels" />
      <Tab label="Invoices" />
      <Tab label="Onchain" />
      <svelte:fragment slot="content">
        <TabContent><Channels {tag} {type} /></TabContent>
        <TabContent>
          <Invoices {tag} {type} />
        </TabContent>
        <TabContent>
          <Onchain {tag} {type} />
        </TabContent>
      </svelte:fragment>
    </Tabs>
  </div>
{/if}

<style>
  .node-url {
    color: #ccc;
    font-weight: bold;
    font-size: 0.75rem;
    height: 1.7rem;
    width: 100%;
    display: flex;
    align-items: center;
    padding: 0 1.4rem;
  }
  .node-url span:first-child {
    margin-right: 1rem;
  }
  .node-url span:last-child {
    transform-origin: center center;
    cursor: pointer;
  }
  .node-url span:last-child:hover {
    color: white;
  }
  .hsmd-wrap {
    width: 100%;
  }
</style>
