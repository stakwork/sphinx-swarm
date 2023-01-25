<script lang="ts">
  import { onMount } from "svelte";

  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import View from "carbon-icons-svelte/lib/List.svelte";
  import { channels, peers as peersStore, balances } from "../store";
  import Peers from "./Peers.svelte";
  import AddChannel from "./AddChannel.svelte";
  import { formatSatsNumbers } from "../helpers";
  import ChannelRows from "./ChannelRows.svelte";

  import {
    get_info,
    list_channels,
    list_peers,
    type LndInfo,
    type Peer,
  } from "../api/lnd";

  export let tag = "";

  $: {
    setup(tag);
  }

  $: peers = $peersStore && $peersStore[tag];

  type ChannelPage = "main" | "peers" | "add_channel";
  let page: ChannelPage = "main";

  let lndData: LndInfo;

  let activePeer: Peer = null;

  async function getLndInfo() {
    const lndRes = await get_info(tag);
    lndData = lndRes;
  }

  async function listChannels() {
    if ($channels[tag] && $channels[tag].length) return;
    const channelsData = await list_channels(tag);

    channels.update((chans) => {
      return { ...chans, [tag]: channelsData };
    });
  }

  async function listPeers() {
    if (peers) return;
    const peersData = await list_peers(tag);
    peersStore.update((peer) => {
      return { ...peer, [tag]: peersData.peers };
    });
  }

  async function setup(_tag) {
    await getLndInfo();
    await listChannels();
    await listPeers();
  }

  function toggleAddPeer() {
    if (page === "peers") {
      page = "main";
    } else {
      page = "peers";
    }
  }

  function toggleAddChannel() {
    if (page === "add_channel") {
      page = "main";
    } else {
      page = "add_channel";
    }
  }

  function formatPubkey(pk: string) {
    return `${pk.substring(0, 6)}...${pk.substring(pk.length - 6)}`;
  }

  let copied = false;
  function copyPubkey() {
    navigator.clipboard.writeText(lndData.identity_pubkey);
    copied = true;
    setTimeout(() => (copied = false), 350);
  }

  function peerAddChannel(peer: Peer) {
    activePeer = peer;
    toggleAddChannel();
  }
</script>

<div class="wrap">
  <section class="header-btns">
    {#if lndData && lndData.identity_pubkey}
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <div class="pubkey" on:click={copyPubkey}>
        {formatPubkey(lndData.identity_pubkey)}
      </div>
    {/if}

    <Button
      kind="tertiary"
      type="submit"
      size="field"
      icon={View}
      disabled={false}
      on:click={toggleAddPeer}
    >
      Peers
    </Button>

    <Button
      kind="tertiary"
      type="submit"
      size="field"
      icon={Add}
      class="channel"
      disabled={false}
      on:click={toggleAddChannel}
    >
      Channel
    </Button>
  </section>

  <section class="liquidity-wrap">
    <aside>
      <h6 class="title">TOTAL INBOUND LIQUIDITY</h6>
      <h3 class="value">
        {formatSatsNumbers($balances.inbound)} <span>SAT</span>
      </h3>
    </aside>
    <aside>
      <h6 class="title">TOTAL OUTBOUND LIQUIDITY</h6>
      <h3 class="value">
        {formatSatsNumbers($balances.outbound)} <span>SAT</span>
      </h3>
    </aside>
  </section>

  {#if page === "peers"}
    <Peers back={toggleAddPeer} {tag} newChannel={peerAddChannel} />
  {:else if page === "add_channel"}
    <AddChannel
      back={toggleAddChannel}
      activeKey={activePeer ? activePeer.pub_key : ""}
      {tag}
    />
    <div />
  {:else if $channels.hasOwnProperty(tag) && $channels[tag].length}
    <ChannelRows {tag} />
  {:else}
    <section class="no-data-wrap">
      <h3>
        No available channels, click on the add channel button to create one.
      </h3>
    </section>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
  }
  .liquidity-wrap {
    background-color: #101317;
    padding: 25px 30px;
    display: flex;
  }

  .liquidity-wrap aside {
    text-align: center;
    width: 50%;
  }

  .liquidity-wrap aside:first-child {
    border-right: 1px solid #6a6d6c;
  }

  .liquidity-wrap aside .title {
    font-size: 0.85rem;
    color: #6b7a8d;
  }

  .liquidity-wrap aside .value {
    font-size: 1.6rem;
    color: #ffffff;
    margin-top: 10px;
  }

  .liquidity-wrap aside .value span {
    color: #6b7a8d;
  }
  .header-btns {
    display: flex;
    margin-left: auto;
    position: absolute;
    right: 1rem;
    top: -5.6rem;
  }
  .no-data-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 50vh;
    width: 100%;
  }
  .no-data-wrap h3 {
    font-size: 0.9rem;
  }
  .pubkey {
    font-size: 0.8rem;
    width: 100%;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
    display: flex;
    align-items: center;
    padding-left: 1rem;
    color: #ddd;
    margin-right: 1rem;
    cursor: pointer;
  }
  .pubkey:hover {
    color: white;
  }
</style>
