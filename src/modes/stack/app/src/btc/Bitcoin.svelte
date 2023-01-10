<script>
  import * as api from "../api";
  import { onMount } from "svelte";
  import { btcinfo, walletBalance } from "../store";
  import BitcoinMine from "./BitcoinMine.svelte";
  import { convertBtcToSats, formatSatsNumbers } from "../helpers";

  export let tag = "";
  let loading = true;

  async function getBitcoinInfo() {
    loading = true;
    
    if ($btcinfo && $btcinfo.blocks) {
      loading = false;
      return;
    }

    btcinfo.set(await api.btc.get_info(tag));
    loading = false;
  }

  async function getWalletBalance() {
    if ($walletBalance) return;

    btcinfo.set(await api.btc.get_balance(tag));
  }

  onMount(() => {
    getBitcoinInfo();
    getWalletBalance();
  });
</script>

<div class="bitcoin-wrapper">
  <h5 class="info">Bitcoin Info</h5>
  <div class="spacer" />
  {#if loading}
    <div class="loading-wrap">
      <h5>Loading Bitcoin Info .....</h5>
    </div>
  {:else if $btcinfo}
    <section class="value-wrap">
      <h3 class="title">NETWORK</h3>
      <h3 class="value">{$btcinfo.chain}</h3>
    </section>
    <section class="value-wrap">
      <h3 class="title">BLOCK HEIGHT</h3>
      <h3 class="value">{$btcinfo.blocks}</h3>
    </section>
    <section class="value-wrap">
      <h3 class="title">BLOCK HEADERS</h3>
      <h3 class="value">{$btcinfo.headers}</h3>
    </section>
    {#if $btcinfo.chain === "regtest"}
      <section class="value-wrap">
        <h3 class="title">WALLET BALANCE</h3>
        <h3 class="value">
          {formatSatsNumbers(convertBtcToSats($walletBalance))} Sats
        </h3>
      </section>

      <BitcoinMine {tag} />
    {/if}
  {/if}
</div>

<style>
  .bitcoin-wrapper {
    padding: 0px 25px;
  }
  .bitcoin-wrapper .info {
    font-size: 1rem;
    margin-bottom: 30px;
  }
  .value-wrap {
    display: flex;
    align-items: center;
    margin-bottom: 20px;
  }
  .title {
    color: #c6c6c6;
    font-size: 0.75rem;
  }
  .value {
    font-size: 0.85rem;
    margin-left: auto;
    font-weight: 600;
  }
</style>
