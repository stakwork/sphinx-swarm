<script>
  import { onMount } from "svelte";

  import ReceiveLineWrap from "../reusable/ReceiveLineWrap.svelte";
  import ReceiveLine from "../reusable/ReceiveLine.svelte";
  import DotWrap from "../reusable/DotWrap.svelte";
  import Dot from "../reusable/Dot.svelte";
  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { channels, balances } from "../store";
  import AddPeer from "./AddPeer.svelte";
  import AddChannel from "./AddChannel.svelte";

  import { get_info, list_channels } from "../api/lnd";

  export let tag = "";

  let add_peer = false;
  let add_channel = false;

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

  function formatSatNumbers(num) {
    const numFormat = new Intl.NumberFormat().format(num).replaceAll(",", " ");
    return numFormat;
  }

  onMount(async () => {
    await getLndInfo();
    await listChannels();
  });

  const mockChannels = [
    {
      active: true,
      remote_pubkey:
        "0350587f325dcd6eb50b1c86874961c134be3ab2b9297d88e61443bb0531d7798e",
      capacity: 100000,
      local_balance: 6935,
      remote_balance: 86541,
    },
    {
      active: true,
      remote_pubkey:
        "023d70f2f76d283c6c4e58109ee3a2816eb9d8feb40b23d62469060a2b2867b77f",
      capacity: 500000,
      local_balance: 218986,
      remote_balance: 280288,
    },
    {
      active: false,
      remote_pubkey:
        "023d70f2f76d283c6c4e58109ee3b1815eb9d8feb40b23d62469060a2b2867b55e",
      capacity: 400000,
      local_balance: 180000,
      remote_balance: 200000,
    },
    {
      active: false,
      remote_pubkey:
        "023d70f2f76d283c6c4e58109ee3b1815eb9d8feb40b23d62469060a2b2867b77f",
      capacity: 450000,
      local_balance: 18986,
      remote_balance: 200288,
    },
  ];

  function toggleAddPeer() {
    add_peer = !add_peer;
  }

  function toggleAddChannel() {
    add_channel = !add_channel;
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
        {formatSatNumbers($balances.inbound)} <span>SAT</span>
      </h3>
    </aside>
    <aside>
      <h6 class="title">TOTAL OUTBOUND LIQUIDITY</h6>
      <h3 class="value">
        {formatSatNumbers($balances.outbound)} <span>SAT</span>
      </h3>
    </aside>
  </section>

  {#if add_peer}
    <AddPeer {toggleAddPeer}/>
  {:else if add_channel}
    <AddChannel {toggleAddChannel}/>
  {:else}
    <section class="lnd-table-wrap">
      <table>
        <thead>
          <th />
          <th>CAN RECEIVE</th>
          <th>CAN SEND</th>
          <th>PEER / ALIAS</th>
        </thead>
        <tbody>
          {#each mockChannels as chan}
            <tr>
              <td>
                <DotWrap>
                  <Dot color={chan.active ? '#52B550' : `#ED7474`} />
                </DotWrap>
              </td>
              <td>
                <section class="can-receive-wrap">
                  {formatSatNumbers(chan.remote_balance)}
                  <ReceiveLineWrap>
                    <ReceiveLine color={"#ED7474"} />
                    <ReceiveLine color={"#ED7474"} width={"80%"} />
                  </ReceiveLineWrap>
                </section>
              </td>
              <td>{formatSatNumbers(chan.local_balance)}</td>
              {#if chan.alias}
                <td>{chan.alias}</td>
              {:else}
                <td>{""}</td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
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
    top: -3.3rem;
  }
</style>
