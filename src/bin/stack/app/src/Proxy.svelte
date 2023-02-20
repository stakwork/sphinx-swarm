<script>
  import { onMount } from "svelte";
  import { formatMillisatsToSats } from "./helpers";
  import { get_proxy_balances } from "./api/proxy";
  import { proxy } from "./store";

  export let tag = "";

  async function getBalances() {
    if ($proxy.total && $proxy.user_count) return;
    proxy.set(await get_proxy_balances(tag));
  }

  onMount(() => {
    getBalances();
  });
</script>

<div class="proxy-wrapper">
  <h5 class="info">Proxy Stats</h5>
  <div class="spacer" />
  {#if $proxy}
    <section class="value-wrap">
      <h3 class="title">TOTAL USERS</h3>
      <!--
        ?? Added to prevent undefined when proxy node is stopped
      -->
      <h3 class="value">{$proxy.user_count ?? 0}</h3>
    </section>
    <section class="value-wrap">
      <h3 class="title">TOTAL SATS BALANCE</h3>
      <h3 class="value">{formatMillisatsToSats($proxy.total)}</h3>
    </section>
  {/if}
</div>

<style>
  .proxy-wrapper {
    padding: 0px 25px;
  }
  .proxy-wrapper .info {
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
