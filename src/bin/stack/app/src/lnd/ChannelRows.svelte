<script lang="ts">
  import ReceiveLineWrap from "../components/ReceiveLineWrap.svelte";
  import ReceiveLine from "../components/ReceiveLine.svelte";
  import DotWrap from "../components/DotWrap.svelte";
  import Dot from "../components/Dot.svelte";
  import { channels } from "../store";
  import { formatSatsNumbers } from "../helpers";

  export let tag = "";

  function getBarCalculation(chan) {
    const remote_balance = Number(chan.remote_balance);
    const local_balance = Number(chan.local_balance);
    const total = remote_balance + local_balance;
    const remote_percentage = Math.floor((remote_balance * 100) / total);
    const local_percentage = Math.floor((local_balance * 100) / total);

    let color = "#52B550";
    if (local_percentage <= 10 || remote_percentage <= 10) {
      color = "#ED7474";
    } else if (local_percentage <= 20 || remote_percentage <= 20) {
      color = "#F2BC52";
    }
    return {
      ...chan,
      color,
      remote_percentage,
      local_percentage,
    };
  }
</script>

<div class="lnd-table-wrap">
  <section class="table-head">
    <div class="th" />
    <div class="th">CAN SEND</div>
    <div class="th">CAN RECEIVE</div>
    <div class="th">PEER / ALIAS</div>
  </section>

  <section class="table-body">
    {#each $channels[tag].map(getBarCalculation) as chan}
      <section class="row">
        <div class="td">
          <DotWrap>
            <Dot color={chan.active ? "#52B550" : `#ED7474`} />
          </DotWrap>
        </div>
        <div class="td">
          <section class="can-receive-wrap">
            <section>
              {formatSatsNumbers(chan.local_balance)}
            </section>
            <ReceiveLineWrap>
              <ReceiveLine
                color={chan.color}
                width={`${chan.local_percentage}%`}
              />
              <ReceiveLine
                color={chan.color}
                width={`${chan.remote_percentage}%`}
              />
            </ReceiveLineWrap>
          </section>
        </div>
        <div class="td">{formatSatsNumbers(chan.remote_balance)}</div>
        <div class="td">
          <span class="pubkey">{chan.remote_pubkey}</span>
        </div>
      </section>
    {/each}
  </section>
</div>

<style>
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
