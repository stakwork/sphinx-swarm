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
  {#each btcInfoData as btc}
    <h3>{btc.chain}</h3>
    <h3>{btc.blocks}</h3>
    <h3>{btc.pruned}</h3>
    <h3>{btc.initialblockdownload}</h3>
  {/each}
</div>

<style>
  .bitcoin-wrapper {
    padding: 0px 20px;
  }
</style>
