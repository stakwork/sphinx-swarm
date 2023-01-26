<script lang="ts">
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import * as LND from "../api/lnd";
  import QrCode from "svelte-qrcode";

  export let tag = "";
  $: amount = 0;
  $: pay_req = "";

  $: invDisabled = !amount;

  async function newInvoice() {
    const invoice = await LND.add_invoice(tag, amount);
    if (invoice) {
      pay_req = invoice.payment_request;
    }
    // console.log("Invoice ===", await LND.add_invoice(tag, amount));
  }

  function copyToClipboard(value) {
    navigator.clipboard.writeText(value);
  }
</script>

<main>
  <section class="invoice-wrap">
    <TextInput
      labelText={"Amount"}
      placeholder={"Enter invoice amount"}
      type={"number"}
      bind:value={amount}
    />
    <div class="spacer" />

    <center
      ><Button
        kind="tertiary"
        type="submit"
        size="field"
        icon={Add}
        class="channel"
        disabled={invDisabled}
        on:click={newInvoice}
      >
        New Invoice
      </Button>
    </center>
  </section>
  <section class="invoice-data">
    <p class="invoice-title">Invoice QR code</p>
    <QrCode padding={1.5} value={pay_req} />

    <div class="invoice">
      {pay_req}
    </div>

    <button class="invoice-btn" on:click={() => copyToClipboard(pay_req)}>Copy Invoice</button>
  </section>
</main>

<style>
  main {
    padding: 25px 30px;
  }
  .invoice-wrap {
    margin-bottom: 20px;
  }

  .invoice-data {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .invoice {
    overflow: scroll;
    text-overflow: clip;
    height: 110px;
    overflow-wrap: break-word;
    font-size: 0.9rem;
    border: 0.5px solid #FFF;
    min-width: 100%;
    max-width: 100%;
    border-radius: 10px;
    margin-top: 20px;
    padding: 10px;
  }
  .invoice-title {
    margin-bottom: 10px;
    font-size: 0.88rem;
  }

.invoice-btn {
   padding: 12px 15px;
   margin-top: 20px;
   font-size: 0.8rem;
   background-color: transparent;
   color: #FFF;
   outline: 0;
   border: 1px solid #FFF;
}
</style>
