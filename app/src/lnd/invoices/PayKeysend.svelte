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
  import { parseClnListPeerRes } from "../../helpers/cln";
  import { convertSatsToMilliSats } from "../../helpers";

  export let tag = "";
  export let type = "";

  $: dest = "";

  $: amount = 0;

  $: invDisabled = !dest || !amount || (dest && dest.length !== 66);

  let show_notification = false;

  async function payKeysend() {
    if (type === "Cln") {
      const payRes = await CLN.keysend(
        tag,
        dest,
        convertSatsToMilliSats(amount)
      );
      if (payRes) {
        show_notification = true;
        dest = "";
        amount = 0;

        setTimeout(async () => {
          const peersData = await CLN.list_peers(tag);
          const parsedRes = await parseClnListPeerRes(peersData);
          if (!peersData) return;
          channels.update((chans) => {
            return { ...chans, [tag]: parsedRes.channels };
          });
        }, 2000);
      }
    } else {
      const payRes = await LND.keysend(tag, dest, amount);
      if (payRes) {
        show_notification = true;
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
  }
</script>

<main>
  <section class="invoice-wrap">
    {#if show_notification}
      <InlineNotification
        lowContrast
        kind="success"
        title="Success:"
        subtitle="Keysend payment has been made."
        timeout={3000}
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
