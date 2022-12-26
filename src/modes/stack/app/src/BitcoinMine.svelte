<script lang="ts">
  import {
    Button,
    Modal,
    TextInput,
  } from "carbon-components-svelte";
  import * as api from "./api";

  export let tag = "";
  let open = false;
  $: blockLen = 6;
  $: addresss = "";

  async function mine() {
    const result = await api.btc.test_mine(tag, blockLen, addresss);
    if (result) {
        open = false;
    }
  }

  $: ok = blockLen && addresss;
</script>

<section class="mine-blocks-btn">
  <Button on:click={() => (open = true)}>Mine 6 or more Blocks</Button>

  <Modal
    bind:open
    modalHeading="Mine Blocks"
    hasForm={true}
    class="mine-block-modal"
    size="sm"
    primaryButtonText="Mine"
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open = !open)}
    on:submit={mine}
    primaryButtonDisabled={!ok}
  >
    <section class="modal-content">
      <div class="spacer" />
      <TextInput
        labelText={"Blocks"}
        placeholder={"Enter number of blocks"}
        type="number"
        bind:value={blockLen}
      />
      <div class="spacer" />
      <TextInput
        labelText={"Address"}
        placeholder={"Enter address"}
        bind:value={addresss}
      />
      <div class="spacer" />
    </section>
  </Modal>
</section>

<style>
  .mine-blocks-btn {
  }
  .modal-content {
    padding: 0px 1.5rem;
  }
  .spacer {
    height: 1rem;
  }
</style>
