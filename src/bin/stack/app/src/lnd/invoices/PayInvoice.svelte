<script lang="ts">
  import { Button, TextArea } from "carbon-components-svelte";
  import Pay from "carbon-icons-svelte/lib/Money.svelte";
  import * as LND from "../../api/lnd";
  import { channels } from "../../store";

  export let tag = "";

  $: pay_req = "";

  $: invDisabled = !pay_req;

  async function payInvoice() {
    const payRes = await LND.pay_invoice(tag, pay_req);
    if (payRes) {
      pay_req = "";

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
      bind:value={pay_req}
      rows={5}
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
        on:click={payInvoice}
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
