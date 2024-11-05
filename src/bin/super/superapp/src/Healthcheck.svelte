<script lang="ts">
  import CheckmarkFilled from "carbon-icons-svelte/lib/CheckmarkFilled.svelte";
  import WarningFilled from "carbon-icons-svelte/lib/WarningFilled.svelte";

  import { onMount } from "svelte";

  export let host = "";

  type Status = "checking" | "ok" | "warn";
  let status: Status = "checking";

  async function checkStatus() {
    try {
      let url = `https://boltwall.${host}/stats`;
      // custom URLs
      if (!/swarm\d+/.test(host)) {
        url = `https://${host}/api/stats`;
      }
      console.log("URL", url);
      const r = await fetch(url);
      const j = await r.json();
      status = "ok";
    } catch (e) {
      console.warn(e);
      status = "warn";
    }
  }
  onMount(() => {
    setTimeout(() => {
      checkStatus();
    }, Math.random() * 1000);
  });
</script>

{#if status === "checking"}
  <span />
{:else if status === "ok"}
  <CheckmarkFilled color="#4cc9b0" />
{:else}
  <WarningFilled color="#D0342C" />
{/if}
