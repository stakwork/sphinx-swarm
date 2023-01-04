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
      local_balance: 100,
      remote_balance: 96541,
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
      local_balance: 200000,
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

  function getBarCalculation(chan) {
    const remote_balance = Number(chan.remote_balance);
    const local_balance = Number(chan.local_balance);

    const total = remote_balance + local_balance;

    const remote_percentage = Math.floor((remote_balance * 100) / total);
    const local_percentage = Math.floor((local_balance * 100) / total);

    let color = "#F2BC52";

    if (local_percentage === remote_percentage) {
      color = "#52B550";
    } else if (local_percentage <= 10 || remote_percentage <= 10) {
      color = "#ED7474";
    }

    return {
      color,
      remote_percentage,
      local_percentage,
    };
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
    <AddPeer {toggleAddPeer} />
  {:else if add_channel}
    <AddChannel {toggleAddChannel} />
    <div />
  {:else}
    <div class="lnd-table-wrap">
      <section class="table-head">
        <div class="th" />
        <div class="th">CAN RECEIVE</div>
        <div class="th">CAN SEND</div>
        <div class="th">PEER / ALIAS</div>
      </section>

      <section class="table-body">
        {#each mockChannels as chan}
          <section class="row">
            <div class="td">
              <DotWrap>
                <Dot color={chan.active ? "#52B550" : `#ED7474`} />
              </DotWrap>
            </div>
            <div class="td">
              <section class="can-receive-wrap">
                <section class="value">
                  {formatSatNumbers(chan.remote_balance)}
                </section>
                <ReceiveLineWrap>
                  <ReceiveLine
                    color={getBarCalculation(chan).color}
                    width={`${getBarCalculation(chan).local_percentage}%`}
                  />
                  <ReceiveLine
                    color={getBarCalculation(chan).color}
                    width={`${getBarCalculation(chan).remote_percentage}%`}
                  />
                </ReceiveLineWrap>
              </section>
            </div>
            <div class="td">{formatSatNumbers(chan.local_balance)}</div>
            <div class="td">
              <span class="pubkey">{chan.remote_pubkey}</span>
            </div>
          </section>
        {/each}
      </section>
    </div>
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

  .pubkey {
    text-overflow: ellipsis;
  }

  .lnd-table-wrap {
    max-width: 100%;
    min-width: 100%;
    font-family: "Barlow", sans-serif;
    display: flex;
    flex-direction: column;
  }

  .lnd-table-wrap .table-head {
    display: flex;
    flex-direction: row;
    border-bottom: 1px solid #101317;
    box-shadow: 0px 1px 6px rgba(0, 0, 0, 0.25);
  }

  .lnd-table-wrap .table-head .th {
    font-size: 0.72rem;
    color: #6b7a8d;
    min-height: 50px;
    padding: 20px 0px;
    text-align: left;
    font-weight: 600;
  }

  .lnd-table-wrap .table-head .th:first-child {
    min-width: 10%;
  }

  .lnd-table-wrap .table-head .th:nth-child(2) {
    min-width: 48%;
  }

  .lnd-table-wrap .table-head .th:nth-child(3) {
    min-width: 15%;
  }

  .lnd-table-wrap .table-head .th:nth-child(4) {
    min-width: 27%;
  }

  .lnd-table-wrap .table-body {
    display: flex;
    flex-direction: column;
    max-width: 100%;
  }

  .lnd-table-wrap .table-body .row {
    display: flex;
    flex-direction: row;
    border-bottom: 1px solid #151e27;
    align-items: center;
  }

  .lnd-table-wrap .table-body .row .td {
    padding: 20px 0px;
    font-size: 0.97rem;
    text-align: left;
    font-weight: 500;
  }

  .lnd-table-wrap .table-body .row .td:first-child {
    min-width: 10%;
  }

  .lnd-table-wrap .table-body .row .td:nth-child(2) {
    min-width: 48%;
  }

  .lnd-table-wrap .table-body .row .td:nth-child(3) {
    min-width: 15%;
  }

  .lnd-table-wrap .table-body .row .td:nth-child(4) {
    min-width: 27%;
  }

  .lnd-table-wrap .table-body .row .td .pubkey {
    width: 9.4vw;
    display: inline-block;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
  }
</style>
