<script lang="ts">
  import { ToastNotification } from "carbon-components-svelte";
  import {
    convertMillisatsToSats,
    formatSatsNumbers,
  } from "../../../../../../app/src/helpers";
  import { splitPubkey } from "../../../../../../app/src/helpers/swarm";
  import type { ILightningBot } from "../types/types";
  import { onMount } from "svelte";

  export let lightningBot: ILightningBot;
  onMount(() => {
    // console.log("tester");
  });
</script>

<div class="bot_card_container">
  {#if lightningBot.error_message}
    <div class="success_toast_container">
      <ToastNotification
        lowContrast
        kind={"error"}
        title={"Error"}
        subtitle={lightningBot.error_message}
        fullWidth={true}
      />
    </div>{:else}
    <div class="bot_card">
      <p>Label: <span class="card_value">{lightningBot.label}</span></p>
      <p>
        Public Key: <span class="card_value"
          >{splitPubkey(lightningBot.contact_info)}</span
        >
      </p>
      <p>
        Balance : <span class="card_value"
          >{formatSatsNumbers(
            convertMillisatsToSats(lightningBot.balance_in_msat)
          )}
          Sats</span
        >
      </p>
      <p>
        Network : <span class="card_value">{lightningBot.network} </span>
      </p>
      <p>
        Alias : <span class="card_value">{lightningBot.alias} </span>
      </p>
    </div>
  {/if}
</div>

<style>
  .bot_card_container {
    border-radius: 1rem;
    min-height: 10rem;
    margin-bottom: 1rem;
    display: flex;
    padding: 1.5rem;
    flex-direction: column;
    border: 1px solid #f7e2e2;
  }

  .card_value {
    font-size: 1.5rem;
  }

  .bot_card {
    display: flex;
    flex-direction: column;
  }
</style>
