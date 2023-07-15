<script>
  import { DataTable, Loading, Pagination } from "carbon-components-svelte";
  import { onMount } from "svelte";
  export let tag = "";
  export let type = "";
  export let paymentType = "";
  import * as CLN from "../api/cln";
  import { parseClnInvoices, parseClnPayments } from "../helpers/cln";

  $: transactions = null;
  let pageSize = 5;
  let page = 1;

  let tempTag = "";
  $: tag, checkTagChange();
  async function getSentPayment() {
    if (type === "Cln") {
      //Make api call to CLN
      const pays = await CLN.list_pays(tag);
      const trans = parseClnPayments(pays.payments);
      transactions = [...trans];
    } else {
      // Make Api call to LND
    }
  }

  function checkTagChange() {
    if (tag !== tempTag) {
      loadTransactions();
      tempTag = tag;
    }
  }

  async function getReceivedPayment() {
    if (type === "Cln") {
      // Make Api call to CLN
      const invoices = await CLN.list_invoices(tag);
      const trans = parseClnInvoices(invoices.invoices);
      transactions = [...trans];
    } else {
      //Make Api call to LND
    }
  }

  function loadTransactions() {
    if (paymentType === "sent") {
      getSentPayment();
    } else {
      getReceivedPayment();
    }
  }

  onMount(() => {
    loadTransactions();
    tempTag = tag;
  });
</script>

<main>
  {#if transactions === null}
    <div class="loader">
      <Loading withOverlay={false} />
      <p>Loading Transactions...</p>
    </div>
  {:else if transactions.length === 0}
    <div class="message">
      <p>No {paymentType} transactions yet!!...</p>
    </div>
  {:else}
    <div>
      <DataTable
        headers={[
          { key: "index", value: "Index" },
          { key: "invoice", value: "Invoice" },
          { key: "date", value: "Date" },
          { key: "amount", value: "Amount" },
        ]}
        rows={transactions}
        {pageSize}
        {page}
      />
      <Pagination
        bind:pageSize
        bind:page
        totalItems={transactions.length}
        pageSizeInputDisabled
      />
    </div>
  {/if}
</main>

<style>
  main {
    margin-top: 1rem;
  }

  .loader {
    margin-top: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
  }

  .loader p {
    margin-top: 1rem;
  }

  .message {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2rem;
  }

  .message p {
    font-size: 1.5rem;
  }
</style>
