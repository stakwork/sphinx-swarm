<script lang="ts">
  import { Button } from "carbon-components-svelte";
  import AddInvoice from "./invoices/AddInvoice.svelte";
  import PayInvoice from "./invoices/PayInvoice.svelte";
  import PayKeysend from "./invoices/PayKeysend.svelte";
  import PaymentHistory from "./invoices/PaymentHistory.svelte";

  type Page = "add" | "pay" | "keysend" | "history";
  let page: Page = "add";

  export let tag = "";
  export let type = "";

  interface Button {
    label: string;
    page: Page;
  }
  const buttons: Button[] = [
    { label: "Add Invoice", page: "add" },
    { label: "Pay Invoice", page: "pay" },
    { label: "Keysend", page: "keysend" },
    { label: "History", page: "history" },
  ];
</script>

<div class="invoice-tabs-wrap">
  {#each buttons as button}
    <Button size="field" kind="tertiary" on:click={() => (page = button.page)}
      >{button.label}</Button
    >
  {/each}
</div>
{#if page === "add"}
  <AddInvoice {tag} {type} />
{:else if page === "pay"}
  <PayInvoice {tag} {type} />
{:else if page === "history"}
  <PaymentHistory {tag} {type} />
{:else}
  <PayKeysend {tag} {type} />
{/if}

<style>
  .invoice-tabs-wrap {
    margin-top: 1rem;
    display: flex;
    justify-content: center;
    align-items: center;
  }
</style>
