<script lang="ts">
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
  } from "../store";
  import { onMount } from "svelte";
  import * as LND from "../api/lnd";
  import * as CLN from "../api/cln";
  import { parseClnGetInfo } from "../helpers/cln";
  import { formatPubkey } from "../helpers";

  export let tag = "";
  export let type = "";
  $: selected = 0;
  $: $finishedOnboarding, selectCurrentTab();
  function selectCurrentTab() {
    // console.log("is onboarding", $isOnboarding);
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

  let lndData: LND.LndInfo;

  $: peering_url = $selectedNode?.host
    ? `${$selectedNode?.host}:${$selectedNode.peer_port}`
    : `${$selectedNode.name}.sphinx:${$selectedNode.peer_port}`;

  let copied = false;
  function copyAddress() {
    navigator.clipboard.writeText(`${lndData?.identity_pubkey}@${peering_url}`);
    copied = true;
    setTimeout(() => (copied = false), 150);
  }

  async function getLndInfo() {
    const lndRes = await LND.get_info(tag);
    lndData = lndRes;
  }

  async function getClnInfo() {
    const clnRes = await CLN.get_info(tag);
    lndData = await parseClnGetInfo(clnRes);
  }

  onMount(() => {
    if (type === "Cln") {
      getClnInfo();
    } else {
      getLndInfo();
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
      <span>Peering Connection String:</span>
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <span
        on:click={copyAddress}
        style={`transform:scale(${copied ? 1.1 : 1});`}
        >{`${formatPubkey(
          lndData?.identity_pubkey || ""
        )}@${peering_url}`}</span
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
