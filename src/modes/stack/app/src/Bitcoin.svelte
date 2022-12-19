<script>
  import { get_info } from "./api/btc";
  import { onMount } from "svelte";
  import { btcinfo } from "./store";
  let btcInfoData = [];
  export let tag = "";

  async function getBitcoinInfo() {
    if (btcinfo && $btcinfo.length) return;

    btcInfoData = [await get_info(tag)];
    btcinfo.set([btcInfoData]);
  }

  onMount(async () => {
    await getBitcoinInfo();
  });
</script>

<div class="bitcoin-wrapper">
  <h5 class="info">Bitcoin Info</h5>
  <div class="spacer" />
  {#each btcInfoData as btc}
    <section class="value-wrap">
      <h3 class="title">NETWORK</h3>
      <h3 class="value">{btc.chain}</h3>
    </section>
    <section class="value-wrap">
      <h3 class="title">BLOCK HEIGHT</h3>
      <h3 class="value">{btc.blocks}</h3>
    </section>
    <section class="value-wrap">
        <h3 class="title">BLOCK HEADERS</h3>
        <h3 class="value">{btc.headers}</h3>
      </section>
    <section class="value-wrap">
      <h3 class="title">PRUNED NODE</h3>
      <h3 class="value">{btc.pruned}</h3>
    </section>
  {/each}
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
