<script lang="ts">
  import {
    Button,
    InlineNotification,
    TextInput,
  } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import * as LND from "../../api/lnd";
  import * as CLN from "../../api/cln";
  import QrCode from "svelte-qrcode";
  import { activeInvoice } from "../../store";
  import { convertSatsToMilliSats } from "../../helpers";

  export let tag = "";
  export let type = "";

  $: amount = 0;

  $: invDisabled = !amount;

  $: invoice = $activeInvoice[tag] || "";
  let message = "";
  let show_notification = false;
  let success = false;

  async function newInvoice() {
    if (type === "Cln") {
      const invoiceRes = await CLN.add_invoice(
        tag,
        convertSatsToMilliSats(amount)
      );
      show_notification = true;
      if (typeof invoiceRes === "string") {
        message = invoiceRes;
        return;
      }
      if (typeof invoiceRes !== "object") {
        message = "invalid response";
        console.log(invoiceRes);
        return;
      }
      if (invoiceRes && invoiceRes.bolt11) {
        success = true;
        message = "Invoice created successfully";
        activeInvoice.update((inv) => {
          return { ...inv, [tag]: invoiceRes.bolt11 };
        });
      }
    } else {
      const invoiceRes = await LND.add_invoice(tag, amount);
      if (invoiceRes) {
        activeInvoice.update((inv) => {
          return { ...inv, [tag]: invoiceRes.payment_request };
        });
      }
    }
  }

  function copyToClipboard(value) {
    navigator.clipboard.writeText(value);
  }
</script>

<main>
  {#if show_notification}
    <InlineNotification
      lowContrast
      kind={success ? "success" : "error"}
      title={success ? "Success:" : "Error:"}
      subtitle={message}
      timeout={9000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  <section class="invoice-wrap">
    <TextInput
      labelText={"Amount (satoshis)"}
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
  {#if invoice}
    <section class="invoice-data">
      <p class="invoice-title">Invoice QR code</p>
      <QrCode size={256} padding={1.5} value={invoice} />

      <div class="invoice">
        {invoice}
      </div>

      <Button
        kind="tertiary"
        class="invoice-btn"
        on:click={() => copyToClipboard(invoice)}>Copy Invoice</Button
      >
    </section>
  {/if}
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
    border: 0.5px solid #fff;
    min-width: 100%;
    max-width: 100%;
    border-radius: 10px;
    margin-top: 20px;
    padding: 10px;
    margin-bottom: 0.7rem;
  }
  .invoice-title {
    margin-bottom: 10px;
    font-size: 0.88rem;
  }
</style>
