<script lang="ts">
  import {
    Button,
    TextInput,
    InlineNotification,
    Modal,
  } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import { add_peer, list_peers, type LndPeer } from "../api/lnd";
  import * as CLN from "../api/cln";
  import {
    peers as peersStore,
    finishedOnboarding,
    createdPeerForOnboarding,
    isOnboarding,
    lightningPeers,
  } from "../store";
  import {
    convertLightningPeersToObject,
    parseClnListPeerRes,
  } from "../helpers/cln";
  import { add_lightning_peer } from "../api/swarm";
  import { formatPubkey, handleGetLightningPeers } from "../helpers/swarm";

  $: pubkey = "";
  $: host = "";
  $: $finishedOnboarding, addDefaultPeer();

  export let back = () => {};
  export let tag = "";
  export let newChannel = (p: LndPeer) => {};
  export let type = "";

  $: peers = $peersStore && $peersStore[tag];

  let show_notification = false;
  let message = "";
  let open_add_peer_detail = false;
  let isSubmitting = false;
  let alias = "";
  let peerPubkey = "";
  let error_notification = false;
  let error_message = false;

  async function addPeer() {
    message = "";
    if (type === "Cln") {
      const peer = await CLN.add_peer(tag, pubkey, host);
      show_notification = true;

      if (typeof peer === "string") {
        message = peer;
        error_message = true;
        return;
      }

      if (typeof peer !== "object") {
        message = "unexpected error";
        error_message = true;
        console.log(peer);
        return;
      }
      if (peer) {
        pubkey = "";
        host = "";
        const peersData = await CLN.list_peers(tag);
        const thepeers = await parseClnListPeerRes(peersData);
        peersStore.update((peer) => {
          return { ...peer, [tag]: thepeers };
        });
        createdPeerForOnboarding.update(() => true);
      }
    } else {
      if (await add_peer(tag, pubkey, host)) {
        show_notification = true;
        pubkey = "";
        host = "";

        setTimeout(async () => {
          const peersData = await list_peers(tag);
          peersStore.update((ps) => {
            return { ...ps, [tag]: peersData.peers };
          });
          createdPeerForOnboarding.update(() => true);
        }, 1000);
      }
    }
  }

  function addDefaultPeer() {
    if (
      $isOnboarding &&
      $finishedOnboarding.hasBalance &&
      !$finishedOnboarding.hasPeers
    ) {
      pubkey =
        "023d70f2f76d283c6c4e58109ee3a2816eb9d8feb40b23d62469060a2b2867b77f";
      host = "54.159.193.149:9735";
    }
  }

  function handleOpenAddPeer() {
    open_add_peer_detail = true;
    alias = "";
    peerPubkey = "";
  }

  function handleOnCloseAddPeer() {
    open_add_peer_detail = false;
    alias = "";
    peerPubkey = "";
  }

  async function handleAddPeer() {
    message = "";
    isSubmitting = true;
    try {
      const res = await add_lightning_peer({ pubkey: peerPubkey, alias });
      message = res.message;
      if (res.success) {
        show_notification = true;
        await handleGetLightningPeers();
        handleOnCloseAddPeer();
        return;
      }
      error_notification = true;
    } catch (error) {
      error_notification = true;
    } finally {
      isSubmitting = false;
    }
  }

  $: peersLength = peers && peers.length ? peers.length : "No";
  $: peersLabel = peers && peers.length <= 1 ? "peer" : "peers";
  $: addDisabled = !pubkey || !host;
  $: peerObj = convertLightningPeersToObject($lightningPeers);

  function formatPubkeyAliasDisplay(pubkey: string, alias: string) {
    return `${alias} (${formatPubkey(pubkey)})`;
  }
</script>

<section class="peer-wrap">
  <div class="header_container">
    <div class="back" on:click={back} on:keypress={() => {}}>
      <ArrowLeft size={24} />
    </div>
    <Button on:click={handleOpenAddPeer}>Add New Peer Detail</Button>
  </div>

  {#if peers && peers.length}
    <div class="label peers-label">{`${peersLength} ${peersLabel}`}</div>
    <div class="peer-list">
      {#each peers as peer}
        <div class="peer">
          <div class="peer-pubkey">
            {`${peerObj[peer.pub_key] ? formatPubkeyAliasDisplay(peer.pub_key, peerObj[peer.pub_key]) : peer.pub_key}`}
          </div>
          <div class="peer-address">{peer.address}</div>
          <Button size="small" kind="tertiary" on:click={() => newChannel(peer)}
            >New Channel</Button
          >
        </div>
      {/each}
    </div>
  {/if}

  <div class="label new-peer-label">New Peer</div>
  {#if show_notification}
    <InlineNotification
      lowContrast
      kind={error_message ? "error" : "success"}
      title={error_message ? "Error" : "Success:"}
      subtitle={message || "Pair has been added."}
      timeout={3000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  <section class="new-peer-form">
    <div class="spacer" />
    <TextInput
      labelText={"Pubkey"}
      placeholder={"New node pubkey"}
      bind:value={pubkey}
    />
    <div class="spacer" />
    <TextInput
      labelText={"Address"}
      placeholder={"New node address"}
      bind:value={host}
    />
    <div class="spacer" />
    <center
      ><Button
        disabled={addDisabled}
        class="peer-btn"
        on:click={addPeer}
        size="field"
        icon={Add}>Add Peer</Button
      ></center
    >
  </section>
  <Modal
    bind:open={open_add_peer_detail}
    modalHeading="Update Swarm"
    primaryButtonDisabled={!peerPubkey || !alias || isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Add Peer"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open_add_peer_detail = false)}
    on:open
    on:close={handleOnCloseAddPeer}
    on:submit={handleAddPeer}
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
        labelText="Alias"
        placeholder="Enter Peer Alias..."
        bind:value={alias}
      />
    </div>
    <div class="input_container">
      <TextInput
        labelText="Pubkey"
        placeholder="Enter Peer Pubkey..."
        bind:value={peerPubkey}
      />
    </div>
  </Modal>
</section>

<style>
  .header_container {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .peer-wrap {
    padding: 20px 30px;
  }
  .back {
    cursor: pointer;
    height: 2rem;
    display: flex;
    align-items: center;
  }
  .label {
    font-size: 1rem;
    margin-top: 1rem;
  }
  .peers-label {
    margin-left: 0rem;
    margin-bottom: 5px;
    font-weight: 800;
  }
  .peer-list {
    display: flex;
    flex-direction: column;
  }
  .peer {
    margin: 0.2rem 0rem;
    display: flex;
    align-items: center;
  }
  .peer-pubkey {
    width: 60%;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
    font-size: 0.95rem;
  }
  .peer-address {
    margin: 0 1rem 0 0.4rem;
    font-size: 0.95rem;
  }

  .input_container {
    margin-bottom: 1rem;
  }
</style>
