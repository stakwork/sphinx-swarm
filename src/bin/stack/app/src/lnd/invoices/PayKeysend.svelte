<script lang="ts">
  import { Button, TextArea, TextInput } from "carbon-components-svelte";
  import Pay from "carbon-icons-svelte/lib/Money.svelte";
  import * as LND from "../../api/lnd";
  import { channels } from "../../store";

  export let tag = "";

  $: dest = "";

  $: amount = 0;

  $: invDisabled = !dest || !amount;

  async function payKeysend() {
    const payRes = await LND.keysend(tag, dest, amount);
    if (payRes) {
      dest = "";
      amount = 0;

      /**
       * After successfully invoice payment fetch the new channels
       * To update the balance
       */
      const channelsData = await LND.list_channels(tag);
      channels.update((chans) => {
        return { ...chans, [tag]: channelsData };
      });
    }
  }
</script>

<main>
  <section class="invoice-wrap">
    <TextArea
      labelText={"Invoice Payment Request"}
      placeholder={"Enter the payment request of the invoice"}
      bind:value={dest}
      rows={2}
    />
    <div class="spacer" />

    <TextInput
      labelText={"Amount"}
      placeholder={"Enter amount"}
      type={"number"}
      bind:value={amount}
    />
    <div class="spacer" />

    <center
      ><Button
        kind="tertiary"
        type="submit"
        size="field"
        icon={Pay}
        class="channel"
        disabled={invDisabled}
        on:click={payKeysend}
      >
        Pay Invoice
      </Button>
    </center>
  </section>
</main>

<style>
  main {
    padding: 25px 30px;
  }
  .invoice-wrap {
    margin-bottom: 20px;
  }
</style>
