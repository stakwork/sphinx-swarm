<script lang="ts">
  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import * as LND from "../api/lnd";

  export let tag = "";

  function newInvoice() {
    // LND....
  }
</script>

<main>
  <Button
    kind="tertiary"
    type="submit"
    size="field"
    icon={Add}
    class="channel"
    disabled={false}
    on:click={newInvoice}
  >
    New Invoice
  </Button>
</main>

<style>
  main {
    padding: 25px 30px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
</style>
