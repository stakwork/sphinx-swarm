<script>
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import { add_peer, list_peers } from "../api/lnd";
  import { peers } from "../store";

  $: pubkey = "";
  $: host = "";

  export let back = () => {};
  export let tag = "";

  async function addPeer() {
    if (await add_peer(tag, pubkey, host)) {
      pubkey = "";
      host = "";

      const peersData = await list_peers(tag);

      peers.update((peer) => {
        return { ...peer, [tag]: peersData.peers };
      });
    }
  }
</script>

<section class="peer-wrap">
  <div class="back" on:click={back} on:keypress={() => {}}>
    <ArrowLeft size={24} />
  </div>
  <section class="peer-content">
    <div class="spacer" />
    <TextInput
      labelText={"Pubkey"}
      placeholder={"Enter node pubkey"}
      bind:value={pubkey}
    />
    <div class="spacer" />
    <TextInput
      labelText={"Host"}
      placeholder={"Enter node host"}
      bind:value={host}
    />
    <div class="spacer" />
    <center
      ><Button class="peer-btn" on:click={addPeer} size="field" icon={Add}
        >Add Peer</Button
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
    width: 2rem;
  }
</style>
