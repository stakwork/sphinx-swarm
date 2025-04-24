<script lang="ts">
  import {
    Button,
    TextInput,
    Dropdown,
    InlineNotification,
  } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import { create_channel, get_balance, list_peers } from "../api/lnd";
  import * as CLN from "../api/cln";
  import { onMount, onDestroy } from "svelte";
  import {
    lndBalances,
    peers as peersStore,
    channels,
    channelCreatedForOnboarding,
    lightningPeers,
  } from "../store";
  import { formatSatsNumbers, convertSatsToMilliSats } from "../helpers";
  import {
    convertLightningPeersToObject,
    parseClnListFunds,
    parseClnListPeerChannelsRes,
    parseClnListPeerRes,
  } from "../helpers/cln";
  import { getLndPendingAndActiveChannels } from "../helpers/lnd";
  import { formatPubkeyAliasDisplay } from "../helpers/swarm";

  export let activeKey: string = null;

  $: pubkey = activeKey ? activeKey : "";
  $: amount = 0;
  $: sats = 0;
  $: peersObj = convertLightningPeersToObject($lightningPeers);

  export let tag = "";
  export let type = "";

  $: balance = $lndBalances.hasOwnProperty(tag) ? $lndBalances[tag] : 0;

  $: addDisabled = !pubkey || !amount || amount > balance;

  $: peers = $peersStore && $peersStore[tag];

  let show_notification = false;

  let peerInterval;

  // Check for length to avoid map error
  $: peerData = peers?.length
    ? peers.map((p) => ({
        id: p.pub_key,
        text: peersObj[p.pub_key]
          ? formatPubkeyAliasDisplay(p.pub_key, peersObj[p.pub_key])
          : p.pub_key,
      }))
    : [];

  /**
   * Add an empty object to avoid udefined displayed when
   * the addchannel is not trigger by clicking on a peer
   */
  $: peerItems = [{ id: "", text: "Select peer" }, ...peerData];

  async function addChannel() {
    if (type === "Cln") {
      const channel = await CLN.create_channel(
        tag,
        pubkey,
        convertSatsToMilliSats(amount),
        sats
      );
      if (channel) {
        show_notification = true;
        pubkey = "";
        amount = 0;
        sats = 0;
        setTimeout(async () => {
          const peersData = await CLN.list_peer_channels(tag);
          const thechans = await parseClnListPeerChannelsRes(peersData);
          channels.update((chans) => {
            return { ...chans, [tag]: thechans };
          });
          await listClnFunds();
          channelCreatedForOnboarding.update(() => true);
        }, 1500);
      }
    } else {
      if (await create_channel(tag, pubkey, amount, sats)) {
        show_notification = true;
        pubkey = "";
        amount = 0;
        sats = 0;
        setTimeout(async () => {
          const channelsData = await getLndPendingAndActiveChannels(tag);
          channels.update((chans) => {
            return { ...chans, [tag]: channelsData };
          });
          await getBalance();
          channelCreatedForOnboarding.update(() => true);
        }, 1500);
      }
    }
  }

  async function getBalance() {
    const balance = await get_balance(tag);
    if (
      lndBalances.hasOwnProperty(tag) &&
      lndBalances[tag] === balance?.confirmed_balance
    )
      return;
    lndBalances.update((n) => {
      return { ...n, [tag]: balance?.confirmed_balance };
    });
  }

  async function listClnFunds() {
    const funds = await CLN.list_funds(tag);
    const balance = parseClnListFunds(funds);
    if (lndBalances.hasOwnProperty(tag) && lndBalances[tag] === balance) return;

    lndBalances.update((n) => {
      return { ...n, [tag]: balance };
    });
  }

  async function getPeers() {
    let newPeers = [];
    if (type === "Cln") {
      const peersData = await CLN.list_peers(tag);
      newPeers = await parseClnListPeerRes(peersData);
    } else {
      const peersData = await list_peers(tag);
      newPeers = peersData.peers;
    }
    if (JSON.stringify(newPeers) !== JSON.stringify(peers)) {
      peersStore.update((ps) => {
        return { ...ps, [tag]: newPeers };
      });
    }
  }

  onMount(() => {
    //Check for peer
    getPeers();

    //pulling for new peer
    peerInterval = setInterval(getPeers, 10000);

    if (type === "Cln") {
      listClnFunds();
    } else {
      getBalance();
    }
  });

  onDestroy(() => {
    if (peerInterval) clearInterval(peerInterval);
  });

  export let back = () => {};
</script>

<section class="channel-wrap">
  <div class="back" on:click={back} on:keypress={() => {}}>
    <ArrowLeft size={24} />
  </div>
  <div class="balance-wrap">
    <section class="value-wrap">
      <h3 class="title">WALLET BALANCE</h3>
      <h3 class="value">{formatSatsNumbers(balance)}</h3>
    </section>
  </div>
  <section class="channel-content">
    {#if show_notification}
      <InlineNotification
        lowContrast
        kind="success"
        title="Success:"
        subtitle="A new channel has been added."
        timeout={3000}
        on:close={(e) => {
          e.preventDefault();
          show_notification = false;
        }}
      />
    {/if}

    <div class="spacer" />
    <Dropdown
      titleText="Peer Pubkey"
      bind:selectedId={pubkey}
      items={peerItems}
    />
    <div class="spacer" />
    <TextInput
      labelText={"Amount (can't be greater than wallet balance)"}
      placeholder={"Enter channel amount"}
      type={"number"}
      bind:value={amount}
    />
    <div class="spacer" />
    <TextInput
      labelText={"Sats per byte"}
      placeholder={"Enter channel sats per byte"}
      type={"number"}
      bind:value={sats}
    />
    <div class="spacer" />
    <center
      ><Button
        disabled={addDisabled}
        class="peer-btn"
        on:click={addChannel}
        size="field"
        icon={Add}>Add Channel</Button
      ></center
    >
  </section>
</section>

<style>
  .channel-wrap {
    padding: 20px 30px;
  }

  .back {
    cursor: pointer;
    height: 2rem;
    display: flex;
    align-items: center;
  }
  .balance-wrap {
    margin-top: 10px;
  }
</style>
