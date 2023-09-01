<script lang="ts">
  import {
    Button,
    TextArea,
    InlineNotification,
  } from "carbon-components-svelte";
  import Pay from "carbon-icons-svelte/lib/Money.svelte";
  import * as LND from "../../api/lnd";
  import * as CLN from "../../api/cln";
  import { channels } from "../../store";
  import { parseClnListPeerRes } from "../../helpers/cln";
  import { getLndPendingAndActiveChannels } from "../../helpers/lnd";

  export let tag = "";
  export let type = "";

  $: pay_req = "";

  $: invDisabled = !pay_req;

  let show_notification = false;
  let message = "";
  let success = false;

  async function payInvoice() {
    if (type === "Cln") {
      const payRes = await CLN.pay_invoice(tag, pay_req);
      show_notification = true;
      if (payRes.status === 0) {
        success = true;
        message = "Invoice payment has been made.";
        pay_req = "";
        setTimeout(async () => {
          const peersData = await CLN.list_peers(tag);
          const res = parseClnListPeerRes(peersData);
          channels.update((chans) => {
            return { ...chans, [tag]: res.channels };
          });
        }, 2000);
      } else {
        success = false;
        pay_req = "";
        if (payRes.status === 1) {
          message = "Invoice payment is pending";
        }
        if (payRes.status === 2) {
          message = "Invoice payment failed";
        }
      }
    } else {
      const payRes = await LND.pay_invoice(tag, pay_req);
      show_notification = true;
      if (!payRes.payment_error) {
        pay_req = "";
        success = true;
        message = "Invoice payment has been made.";
        /**
         * After successfully invoice payment fetch the new channels
         * To update the balance
         */
        const channelsData = await getLndPendingAndActiveChannels(tag);
        channels.update((chans) => {
          return { ...chans, [tag]: channelsData };
        });
      } else {
        success = false;
        message = payRes.payment_error;
        pay_req = "";
      }
    }
  }
</script>

<main>
  <section class="invoice-wrap">
    {#if show_notification}
      <InlineNotification
        lowContrast
        kind={success ? "success" : "error"}
        title={success ? "Success:" : "Error:"}
        subtitle={message}
        timeout={3000}
        on:close={(e) => {
          e.preventDefault();
          show_notification = false;
        }}
      />
    {/if}

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
