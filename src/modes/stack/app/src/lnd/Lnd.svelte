<script lang="ts">
  import { onMount } from "svelte";

  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { channels, balances } from "../store";
  import AddPeer from "./AddPeer.svelte";
  import AddChannel from "./AddChannel.svelte";
  import { formatSatsNumbers } from "../helpers";
  import Channels from "./Channels.svelte";

  import { get_info, list_channels } from "../api/lnd";

  export let tag = "";

  type ChannelPage = "main" | "add_peer" | "add_channel";
  let page: ChannelPage = "main";

  let lndData = {};

  async function getLndInfo() {
    const lndRes = await get_info(tag);
    lndData = lndRes;
  }

  async function listChannels() {
    if ($channels && $channels.length) return;
    const channelsData = await list_channels(tag);
    channels.set(channelsData);
  }

  onMount(async () => {
    await getLndInfo();
    await listChannels();
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

  {#if page === "add_peer"}
    <AddPeer back={toggleAddPeer} />
  {:else if page === "add_channel"}
    <AddChannel back={toggleAddChannel} />
    <div />
  {:else}
    <Channels />
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
    top: -3.3rem;
  }
</style>
