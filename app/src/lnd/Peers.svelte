<script lang="ts">
  import {
    Button,
    TextInput,
    InlineNotification,
  } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import { add_peer, list_peers, type LndPeer } from "../api/lnd";
  import * as CLN from "../api/cln";
  import { peers as peersStore } from "../store";
  import { parseClnListPeerRes } from "../helpers/cln";

  $: pubkey = "";
  $: host = "";

  export let back = () => {};
  export let tag = "";
  export let newChannel = (p: LndPeer) => {};
  export let type = "";

  $: peers = $peersStore && $peersStore[tag];

  let show_notification = false;

  async function addPeer() {
    if (type === "Cln") {
      const peer = await CLN.add_peer(tag, pubkey, host);
      if (peer) {
        show_notification = true;
        pubkey = "";
        host = "";
        const peersData = await CLN.list_peers(tag);
        const parsedRes = await parseClnListPeerRes(peersData);
        peersStore.update((peer) => {
          return { ...peer, [tag]: parsedRes.peers };
        });
      }
    } else {
      if (await add_peer(tag, pubkey, host)) {
        show_notification = true;
        pubkey = "";
        host = "";

        const peersData = await list_peers(tag);

        peersStore.update((ps) => {
          return { ...ps, [tag]: peersData.peers };
        });
      }
    }
  }

  $: peersLength = peers && peers.length ? peers.length : "No";
  $: peersLabel = peers && peers.length <= 1 ? "peer" : "peers";
  $: addDisabled = !pubkey || !host;
</script>

<section class="peer-wrap">
  <div class="back" on:click={back} on:keypress={() => {}}>
    <ArrowLeft size={24} />
  </div>

  {#if peers && peers.length}
    <div class="label peers-label">{`${peersLength} ${peersLabel}`}</div>
    <div class="peer-list">
      {#each peers as peer}
        <div class="peer">
          <div class="peer-pubkey">{peer.pub_key}</div>
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
      kind="success"
      title="Success:"
      subtitle="Pair has been added."
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
</section>

<style>
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
</style>
