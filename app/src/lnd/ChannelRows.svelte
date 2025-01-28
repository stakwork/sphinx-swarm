<script lang="ts">
  import {
    TextInput,
    Button,
    InlineLoading,
    Modal,
    InlineNotification,
  } from "carbon-components-svelte";
  import ReceiveLineWrap from "../components/ReceiveLineWrap.svelte";
  import ReceiveLine from "../components/ReceiveLine.svelte";
  import DotWrap from "../components/DotWrap.svelte";
  import Dot from "../components/Dot.svelte";
  import { channels, lightningPeers, peers } from "../store";
  import { formatSatsNumbers } from "../helpers";
  import { getTransactionStatus, getBlockTip } from "../helpers/bitcoin";
  import Exit from "carbon-icons-svelte/lib/Exit.svelte";
  import { onDestroy, onMount } from "svelte";
  import {
    convertLightningPeersToObject,
    convertPeersToConnectObj,
    fetchAndUpdateClnPeerStore,
  } from "../helpers/cln";
  import { add_peer } from "../api/cln";
  import Toast from "svelte-toast"

  export let tag = "";
  export let type = "";
  export let onclose = (id: string, dest: string) => {};

  let channel_arr = $channels[tag];

  $: peersObj = convertLightningPeersToObject($lightningPeers);

  $: peersConnectObj = convertPeersToConnectObj($peers[tag]);

  function copyText(txt: string) {
    navigator.clipboard.writeText(txt);
  }

  const toast = new Toast({
    position: 'top-center',
    duration: 3000,         
  });

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

  let selectedChannelParter = "";
  let forceCloseDestination = "";
  let open_reconnect_peer = false;
  let isSubmitting = false;
  let reconnectPubkey = "";
  let reconnectHost = "";
  let error_notification = false;
  let message = "";
  let notification_error = false;
  let show_notification = false;

  function clickRow(chan) {
    if (!chan.active) return;
    copyText(chan.remote_pubkey)
    toast.success("Pubkey copied")
    if (selectedChannelParter === chan.remote_pubkey) {
      selectedChannelParter = "";
      forceCloseDestination = "";
    } else {
      // selectedChannelParter = chan.remote_pubkey; // We are stopping the ability to close channel from the UI
      selectedChannelParter = ""
    }
  }

  let closing = false;
  async function forceClose(e) {
    e.stopPropagation();
    closing = true;
    await onclose(selectedChannelParter, forceCloseDestination);
    closing = false;
  }

  async function getConfirmation(chan) {
    try {
      const channel_point_arr = chan.channel_point.split(":");
      if (channel_point_arr.length < 2) {
        return 0;
      }
      let tx_id = channel_point_arr[0];
      const transaction_status = await getTransactionStatus(tx_id);
      if (!transaction_status.confirmed) {
        return 0;
      }
      const currentBlockHeight = await getBlockTip();
      return currentBlockHeight - transaction_status.block_height + 1;
    } catch (e) {
      console.warn(e);
      return 0;
    }
  }

  async function getChannelsConfirmation() {
    let new_channel = [];
    let notActiveExist = false;
    for (const chan of channel_arr) {
      if (!chan.active) {
        notActiveExist = true;
        const confirmation = await getConfirmation(chan);
        new_channel.push({ ...chan, confirmation });
      }
    }
    // if (notActiveExist) {
    //   channel_arr = [...new_channel];
    // }
  }

  function openReconnectPeerModal(e, pubkey) {
    e.stopPropagation();
    if (type !== "Cln") {
      return;
    }
    reconnectPubkey = pubkey;
    open_reconnect_peer = true;
  }

  async function handlePeerReconnect() {
    isSubmitting = true;
    try {
      // reconnet to peer by sending pubkey
      const res = await add_peer(tag, reconnectPubkey, reconnectHost);
      // check response
      if (typeof res === "string") {
        message = res;
        error_notification = true;
        return;
      }
      if (res.id && res.address) {
        // update peers again
        await fetchAndUpdateClnPeerStore(tag);
        reconnectHost = "";
        reconnectPubkey = "";
        open_reconnect_peer = false;
        show_notification = true;
        message = "Peer Reconnected Successfully";
      }
    } catch (error) {
      console.log("Error going well", error);
    } finally {
      isSubmitting = false;
    }
  }

  function handleOnCloseReconnectPeer() {
    reconnectPubkey = "";
    open_reconnect_peer = false;
    reconnectHost = "";
  }

  function handleClickRemotePubkey(pubkey: string) {
    console.log("Our Pubkey",pubkey)
  }

  let chanInterval;

  onMount(() => {
    getChannelsConfirmation();
    chanInterval = setInterval(getChannelsConfirmation, 50000);
  });

  onDestroy(() => {
    if (chanInterval) clearInterval(chanInterval);
  });
</script>

<div class="lnd-table-wrap">
  {#if show_notification}
    <InlineNotification
      kind={notification_error ? "error" : "success"}
      title={notification_error ? "Error:" : "Success:"}
      subtitle={message}
      timeout={8000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  <section class="table-head">
    <div class="th" />
    <div class="th">CAN SEND</div>
    <div class="th">CAN RECEIVE</div>
    <div class="th">PEER / ALIAS</div>
  </section>

  <section class="table-body">
    {#each channel_arr.map(getBarCalculation) as chan}
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <section
        class={`${
          selectedChannelParter === chan.remote_pubkey ? "selected" : ""
        } row`}
        on:click={() => clickRow(chan)}
      >
        <div class="row-top">
          <div class="td">
            <DotWrap>
              <Dot color={chan.active ? "#52B550" : `#ED7474`} />
            </DotWrap>
          </div>
          {#if chan.active}
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
          {:else}
            <div class="inactive">
              Channel Not Active <span class="">
                ({chan["confirmation"] || 0}/6)</span
              >
            </div>
          {/if}
          <div class="td">
            {#if type === "Cln"}
              {#if peersConnectObj[chan.remote_pubkey]}
                <span class="pubkey">
                  {peersObj[chan.remote_pubkey] || chan.remote_pubkey}
                </span>
              {:else}
                <span>
                  <Button
                    skeleton={false}
                    on:click={(e) =>
                      openReconnectPeerModal(e, chan.remote_pubkey)}
                    size="small"
                    kind={"tertiary"}
                    >Reconnect{peersObj[chan.remote_pubkey]
                      ? ` to ${peersObj[chan.remote_pubkey]}`
                      : ""}</Button
                  >
                </span>
              {/if}
            {:else}
              <span class="pubkey">
                {peersObj[chan.remote_pubkey] || chan.remote_pubkey}
              </span>
            {/if}
          </div>
        </div>
        {#if selectedChannelParter === chan.remote_pubkey}
          <div class="row-bottom">
            <div
              class="row-bottom-scid"
              on:click|stopPropagation={() => copyText(chan.chan_id)}
            >
              {chan.chan_id}
            </div>
            <div class="row-bottom-text">
              <TextInput
                size="sm"
                placeholder={"Close Channel To Address"}
                bind:value={forceCloseDestination}
                on:click={(e) => e.stopPropagation()}
              />
            </div>
            <Button
              disabled={!forceCloseDestination}
              on:click={forceClose}
              size="small"
              kind="danger-tertiary"
              icon={Exit}
            >
              Close
            </Button>
            {#if closing}
              <div class="loading-wrapper">
                <InlineLoading />
              </div>
            {/if}
          </div>
        {/if}
      </section>
    {/each}
  </section>
  <Modal
    bind:open={open_reconnect_peer}
    modalHeading={"Reconnect Peer"}
    primaryButtonDisabled={!reconnectPubkey || !reconnectHost || isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Reconnect Peer"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open_reconnect_peer = false)}
    on:open
    on:close={handleOnCloseReconnectPeer}
    on:submit={handlePeerReconnect}
  >
    {#if error_notification}
      <InlineNotification
        kind="error"
        title="Error:"
        subtitle={message}
        timeout={8000}
        on:close={(e) => {
          e.preventDefault();
          error_notification = false;
        }}
      />
    {/if}
    <div class="input_container">
      <TextInput
        labelText="Pubkey"
        placeholder="Enter Pubkey"
        bind:value={reconnectPubkey}
        readonly={true}
      />
    </div>
    <div class="ip_address_container">
      <TextInput
        labelText="IP Address"
        placeholder="Enter IP Address..."
        bind:value={reconnectHost}
      />
    </div>
  </Modal>
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

  .ip_address_container {
    margin-top: 1.5rem;
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
    flex-direction: column;
    border-bottom: 1px solid #151e27;
    cursor: pointer;
    height: 62px;
  }

  .lnd-table-wrap .table-body .row.selected {
    height: 124px;
  }

  .lnd-table-wrap .table-body .row .row-top {
    height: 62px;
    display: flex;
    flex-direction: row;
    align-items: center;
  }

  .lnd-table-wrap .table-body .row .row-bottom {
    height: 62px;
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-around;
  }

  .lnd-table-wrap .table-body .row .row-bottom-scid {
    font-size: 0.8rem;
    max-width: 5rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lnd-table-wrap .table-body .row .row-bottom-text {
    width: 330px;
    min-width: 330px;
    margin: 0 20px;
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
  .loading-wrapper {
    height: 60px;
    width: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-left: 20px;
  }
  .inactive {
    font-size: 16px;
    width: 420px;
  }
</style>
