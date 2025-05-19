<script>
  import { DataTable, Loading, Pagination } from "carbon-components-svelte";
  import { onMount } from "svelte";
  export let tag = "";
  export let type = "";
  export let paymentType = "";
  import * as CLN from "../api/cln";
  import * as LND from "../api/lnd";
  import { shortTransactionId } from "../helpers";
  import { parseClnInvoices, parseClnPayments } from "../helpers/cln";
  import { parseLndPayments, parseLndInvoices } from "../helpers/lnd";

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
      trans.reverse();
      transactions = [...trans];
    } else {
      // Make Api call to LND
      const payments = await LND.list_payments(tag);
      const trans = parseLndPayments(payments);
      transactions = [...trans];
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
      const invoices = await LND.list_invoices(tag);
      const trans = parseLndInvoices(invoices);
      transactions = [...trans];
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
      >
        <svelte:fragment slot="cell" let:cell>
          {#if cell.key === "invoice"}
            <div>
              {shortTransactionId(cell.value)}
              <button
                class="button"
                on:click={() => navigator.clipboard.writeText(cell.value)}
              >
                Copy
              </button>
            </div>
          {:else}
            {cell.value}
          {/if}
        </svelte:fragment>
      </DataTable>
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
  .button {
    display: inline-block;
    padding: 8px 16px;
    background-color: #636363;
    color: white;
    border: none;
    border-radius: 10px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    text-align: center;
    transition: background-color 0.2s ease;
  }

  .button:hover {
    background-color: #222222; 
  }

  .button:active {
    background-color: #004085;
  }

  .button:focus {
    outline: 2px solid #0056b3;
    outline-offset: 2px;
  }

  .button:disabled {
    background-color: #6c757d;
    cursor: not-allowed;
  }
</style>
