<script lang="ts">
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import { create_channel } from "../api/lnd";

  export let activeKey: string = null;

  $: pubkey = activeKey ? activeKey : "";
  $: amount = 0;
  $: sats = 0;

  export let tag = "";

  $: addDisabled = !pubkey || !amount;

  async function addChannel() {
    if (await create_channel(tag, pubkey, amount, sats)) {
      pubkey = "";
      amount = 0;
      sats = 0;
    }
  }

  export let back = () => {};
</script>

<section class="channel-wrap">
  <div class="back" on:click={back} on:keypress={() => {}}>
    <ArrowLeft size={24} />
  </div>
  <section class="channel-content">
    <div class="spacer" />
    <TextInput
      labelText={"Peer Pubkey"}
      placeholder={"Enter node pubkey"}
      bind:value={pubkey}
    />
    <div class="spacer" />
    <TextInput
      labelText={"Amount"}
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
</style>
