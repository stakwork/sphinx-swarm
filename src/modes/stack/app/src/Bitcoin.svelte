<script>
  import * as api from "./api";
  import { onMount } from "svelte";
  import { btcinfo } from "./store";
  import BitcoinMine from "./BitcoinMine.svelte";

  export let tag = "";

  async function getBitcoinInfo() {
    if ($btcinfo && $btcinfo.length) return;
    btcinfo.set(await api.btc.get_info(tag));
  }

  onMount(() => {
    getBitcoinInfo();
  });

</script>

<div class="bitcoin-wrapper">
  <h5 class="info">Bitcoin Info</h5>
  <div class="spacer" />
  {#if $btcinfo}
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
    <section class="value-wrap">
      <h3 class="title">PRUNED NODE</h3>
      <h3 class="value">{$btcinfo.pruned}</h3>
    </section>
  {/if}
  <BitcoinMine {tag}/>
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
