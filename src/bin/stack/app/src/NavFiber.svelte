<script>
  import { onMount } from "svelte";
  import { get_proxy_balances } from "./api/proxy";
  import { proxy } from "./store";
  import { Button } from "carbon-components-svelte";


  export let tag = "";
  export let host = "";
  let link = host ? host : "http://localhost:80";

  async function getBalances() {
    if ($proxy.total && $proxy.user_count) return;
    proxy.set(await get_proxy_balances(tag));
  }

  onMount(() => {
    getBalances();
  });
</script>

<div class="nav-wrapper">
   <Button target="_blank" href={link}>Open Second Brain</Button>
</div>

<style>
  .nav-wrapper {
    padding: 0px 25px;
  }
</style>
