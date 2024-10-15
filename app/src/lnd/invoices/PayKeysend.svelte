<script lang="ts">
  import {
    Button,
    TextInput,
    InlineNotification,
  } from "carbon-components-svelte";
  import Pay from "carbon-icons-svelte/lib/Money.svelte";
  import * as LND from "../../api/lnd";
  import { channels } from "../../store";
  import * as CLN from "../../api/cln";
  import { parseClnListPeerChannelsRes } from "../../helpers/cln";
  import { convertSatsToMilliSats } from "../../helpers";
  import { getLndPendingAndActiveChannels } from "../../helpers/lnd";

  export let tag = "";
  export let type = "";

  $: dest = "";

  $: amount = 0;

  $: invDisabled = !dest || !amount || (dest && dest.length !== 66);

  let show_notification = false;
  let payment_error = "";

  async function payKeysend() {
    if (type === "Cln") {
      const payRes = await CLN.keysend(
        tag,
        dest,
        convertSatsToMilliSats(amount),
        window.route_hint,
        window.maxfeepercent,
        window.exemptfee
      );
      if (payRes) {
        show_notification = true;
        payment_error = "";
        dest = "";
        amount = 0;

        setTimeout(async () => {
          const peersData = await CLN.list_peer_channels(tag);
          const thechans = await parseClnListPeerChannelsRes(peersData);
          if (!peersData) return;
          channels.update((chans) => {
            return { ...chans, [tag]: thechans };
          });
        }, 2000);
      } else {
        show_notification = true;
        payment_error = "keysend was declined";
      }
    } else {
      // window.tlvs = {133773310:Array(320).fill(9)}
      const payRes = await LND.keysend(tag, dest, amount, window.tlvs);
      if (payRes) {
        if (payRes.payment_error) {
          payment_error = payRes.payment_error;
        } else {
          payment_error = "";
        }
        show_notification = true;
        dest = "";
        amount = 0;
        /**
         * After successfully invoice payment fetch the new channels
         * To update the balance
         */
        const channelsData = await getLndPendingAndActiveChannels(tag);
        channels.update((chans) => {
          return { ...chans, [tag]: channelsData };
        });
      }
    }
  }
</script>

<main>
  <section class="invoice-wrap">
    {#if show_notification}
      <InlineNotification
        lowContrast
        kind={payment_error ? "error" : "success"}
        title={payment_error ? "Failure:" : "Success:"}
        subtitle={payment_error || "Keysend payment has been made."}
        timeout={4000}
        on:close={(e) => {
          e.preventDefault();
          show_notification = false;
        }}
      />
    {/if}

    <TextInput
      labelText={"Pubkey"}
      placeholder={"Destintaion Public Key"}
      bind:value={dest}
    />
    <div class="spacer" />

    <TextInput
      labelText={"Amount (satoshis)"}
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
        Pay Keysend
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
