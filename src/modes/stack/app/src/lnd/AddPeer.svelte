<script>
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";

  $: pubkey = "";
  $: host = "";

  async function addPeer() {}
</script>

<section class="peer-wrap">
  <section class="peer-content">
    <div class="spacer" />
    <TextInput
      labelText={"Pubkey"}
      placeholder={"Enter node pubkey"}
      type="number"
      bind:value={pubkey}
    />
    <div class="spacer" />
    <TextInput
      labelText={"Host"}
      placeholder={"Enter node host"}
      bind:value={host}
    />
    <div class="spacer" />
    <center><Button class="peer-btn" on:click={addPeer} size="field" icon={Add}>Add Peer</Button></center>
  </section>
</section>

<style>
  .peer-wrap {
    padding: 20px 30px;
  }
</style>
