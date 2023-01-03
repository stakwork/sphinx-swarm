<script>
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";

  $: pubkey = "";
  $: amount = "";
  $: sats = "";

  async function addChannel() {}
</script>

<section class="channel-wrap">
  <section class="channel-content">
    <div class="spacer" />
    <TextInput
      labelText={"Pubkey"}
      placeholder={"Enter node pubkey"}
      type="number"
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
      ><Button class="peer-btn" on:click={addChannel} size="field" icon={Add}
        >Add Channel</Button
      ></center
    >
  </section>
</section>

<style>
  .channel-wrap {
    padding: 20px 30px;
  }
</style>
