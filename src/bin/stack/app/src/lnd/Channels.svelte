<script lang="ts">
  import { onMount } from "svelte";

  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import View from "carbon-icons-svelte/lib/List.svelte";
  import { channels, peers } from "../store";
  import AddPeer from "./AddPeer.svelte";
  import AddChannel from "./AddChannel.svelte";
  import { formatSatsNumbers } from "../helpers";
  import ChannelRows from "./ChannelRows.svelte";

  import { get_info, list_channels, list_peers } from "../api/lnd";
  import { derived } from "svelte/store";

  export let tag = "";

  type ChannelPage = "main" | "add_peer" | "add_channel";
  let page: ChannelPage = "main";

  let lndData = {};

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
    if ($peers[tag] && $peers[tag].length) return;
    const peersData = await list_peers(tag);

    peers.update((peer) => {
      return { ...peer, [tag]: peersData.peers };
    });
  }

  onMount(async () => {
    await getLndInfo();
    await listChannels();
    await listPeers();
  });

  function toggleAddPeer() {
    if (page === "add_peer") {
      page = "main";
    } else {
      page = "add_peer";
    }
  }

  function toggleAddChannel() {
    if (page === "add_channel") {
      page = "main";
    } else {
      page = "add_channel";
    }
  }

  $: balances = derived(channels, ($channels) => ({
    inbound:
      $channels[tag] && $channels[tag].length
        ? $channels[tag].reduce((acc, chan) => acc + chan.remote_balance, 0)
        : 0,
    outbound:
      $channels[tag] && $channels[tag].length
        ? $channels[tag].reduce((acc, chan) => acc + chan.local_balance, 0)
        : 0,
  }));

  $: totalPeers = $peers.hasOwnProperty(tag) ? $peers[tag].length : 0;
</script>

<div class="wrap">
  <section class="header-btns">
    <Button
      kind="tertiary"
      type="submit"
      size="field"
      icon={Add}
      disabled={false}
      on:click={toggleAddPeer}
    >
      Add Peer
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
      Add Channel
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

  <section class="peers">
    <Button
      kind="tertiary"
      type="submit"
      size="field"
      icon={View}
      disabled={false}
      on:click={() => {}}
    >
      Total Peers ({totalPeers})
    </Button>
  </section>
  <section class="divider" />

  {#if page === "add_peer"}
    <AddPeer back={toggleAddPeer} {tag} />
  {:else if page === "add_channel"}
    <AddChannel back={toggleAddChannel} {tag} />
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

  .peers {
    align-items: center;
    padding: 20px 25px;
    padding-bottom: 5px;
  }
</style>
