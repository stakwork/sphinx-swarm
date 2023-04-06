<script lang="ts">
  import { Tabs, Tab, TabContent, Button } from "carbon-components-svelte";
  import AddInvoice from "./invoices/AddInvoice.svelte";
  import PayInvoice from "./invoices/PayInvoice.svelte";
  import PayKeysend from "./invoices/PayKeysend.svelte";

  type Page = "add" | "pay" | "keysend";
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
  ];
</script>

<div class="invoice-tabs-wrap">
  {#each buttons as button}
    <Button size="field" kind="tertiary" on:click={() => (page = button.page)}
      >{button.label}</Button
    >
  {/each}

  <!-- <Tabs>
    <Tab label="Add Invoice" />
    <Tab label="Pay Invoice" />
    <Tab label="Keysend" />
    <svelte:fragment slot="content">
      <TabContent><AddInvoice {tag} /></TabContent>
      <TabContent><PayInvoice {tag} /></TabContent>
      <TabContent><PayKeysend {tag} /></TabContent>
    </svelte:fragment>
  </Tabs> -->
</div>
{#if page === "add"}
  <AddInvoice {tag} {type} />
{:else if page === "pay"}
  <PayInvoice {tag} {type} />
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
