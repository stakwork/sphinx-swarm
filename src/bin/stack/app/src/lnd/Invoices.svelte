<script lang="ts">
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import AddInvoice from "./invoices/AddInvoice.svelte";
  import PayInvoice from "./invoices/PayInvoice.svelte";
  import PayKeysend from "./invoices/PayKeysend.svelte";

  export let tag = "";
</script>

<div class="invoice-tabs-wrap">
  <Tabs>
    <Tab label="AddInvoice" />
    <Tab label="Pay Invoices" />
    <Tab label="Pay Keysend" />
    <svelte:fragment slot="content">
      <TabContent><AddInvoice {tag} /></TabContent>
      <TabContent><PayInvoice {tag} /></TabContent>
      <TabContent><PayKeysend {tag} /></TabContent>
    </svelte:fragment>
  </Tabs>
</div>

<style>
  .invoice-tabs-wrap {
    margin-top: 5px;
  }
</style>
