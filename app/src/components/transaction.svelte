<script>
  import { DataTable, Loading, Pagination } from "carbon-components-svelte";
  import { onMount } from "svelte";
  export let tag = "";
  export let type = "";
  export let paymentType = "";

  console.log(tag);

  $: transactions = null;
  let pageSize = 5;
  let page = 1;
  let tran = [
    {
      id: "1pYt...PUyt",
      index: "1.",
      invoice: "1pYt...PUyt",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "1pYt...PUut",
      index: "2.",
      invoice: "1pYt...PUyt",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "1pYt...Ppyt",
      index: "3.",
      invoice: "1pYt...PUyt",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "1pYt...lUyt",
      index: "4.",
      invoice: "1pYt...PUyt",
      date: "10 December, 2011",
      amount: "10,000 sats",
    },
    {
      id: "1pYq...PUyt",
      index: "5.",
      invoice: "3pYt...PUyt",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "9pYt...PUyt",
      index: "6.",
      invoice: "1pYt...PUqw",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "9pNt...PUyt",
      index: "7.",
      invoice: "1pYt...PUqw",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "6pYt...PUyt",
      index: "8.",
      invoice: "1pYt...PUqw",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "9qweYt...PUyt",
      index: "9.",
      invoice: "1pYt...PUqw",
      date: "10 March, 2011",
      amount: "10,000 sats",
    },
    {
      id: "5qwet...PUyt",
      index: "10.",
      invoice: "1pYt...PUqw",
      date: "10 November, 2011",
      amount: "10,000 sats",
    },
  ];

  async function getSentPayment() {
    if (type === "Cln") {
      //Make api call to CLN
      setTimeout(() => {
        transactions = [];
      }, 15000);
    } else {
      // Make Api call to LND
    }
  }

  async function getReceivedPayment() {
    if (type === "Cln") {
      // Make Api call to CLN
      setTimeout(() => {
        transactions = [...tran];
      }, 5000);
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
