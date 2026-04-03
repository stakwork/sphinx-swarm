<script lang="ts">
  import { Button, PasswordInput, Loading, InlineNotification } from "carbon-components-svelte";
  import { get_bot_balance, create_bot_invoice, get_bot_token } from "./api/swarm";
  import { formatMillisatsToSats, convertSatsToMilliSats } from "./helpers";
  import QrCode from "svelte-qrcode";
  import { onMount } from "svelte";

  let isLoading = true;
  let adminToken = "";

  let balance: string | number = "";
  let balanceLoading = false;

  let amount = 0;
  let invoiceLoading = false;
  let bolt11 = "";
  let showNotification = false;
  let notificationKind: "error" | "success" = "error";
  let notificationMessage = "";

  $: invDisabled = !amount || amount <= 0 || invoiceLoading;

  async function fetchBalance() {
    balanceLoading = true;
    try {
      const res = await get_bot_balance();
      if (res?.success && res?.data) {
        balance = formatMillisatsToSats(res.data.msat);
      } else {
        balance = "—";
      }
    } catch (e) {
      balance = "—";
    }
    balanceLoading = false;
  }

  async function createInvoice() {
    invoiceLoading = true;
    bolt11 = "";
    showNotification = false;
    try {
      const res = await create_bot_invoice(convertSatsToMilliSats(amount));
      if (res?.success && res?.data) {
        bolt11 = res.data.bolt11 || res.data.invoice || res.data.payment_request || JSON.stringify(res.data);
        notificationKind = "success";
        notificationMessage = "Invoice created successfully";
        showNotification = true;
        await fetchBalance();
      } else {
        notificationKind = "error";
        notificationMessage = res?.message || "Failed to create invoice";
        showNotification = true;
      }
    } catch (e) {
      notificationKind = "error";
      notificationMessage = "An error occurred";
      showNotification = true;
    }
    invoiceLoading = false;
  }

  function copyToClipboard(value: string) {
    navigator.clipboard.writeText(value);
  }

  onMount(async () => {
    const res = await get_bot_token();
    if (res?.success && res?.data) {
      adminToken = res.data;
    }
    isLoading = false;
    await fetchBalance();
  });
</script>

<div class="bot-wrapper">
  {#if isLoading}<Loading />{/if}

  <div class="bot-container">
    <!-- Balance Section -->
    <div class="section">
      <div class="section-title">Balance</div>
      {#if balanceLoading}
        <div class="balance-loading"><Loading small /></div>
      {:else}
        <div class="balance-value">{balance} sats</div>
      {/if}
    </div>

    <!-- Admin Token Section -->
    <div class="section">
      <PasswordInput labelText="Admin Token" value={adminToken} readonly />
    </div>

    <!-- Receive Invoice Section -->
    <div class="section">
      <div class="section-title">Receive (Create Invoice)</div>

      {#if showNotification}
        <InlineNotification
          lowContrast
          kind={notificationKind}
          title={notificationKind === "success" ? "Success:" : "Error:"}
          subtitle={notificationMessage}
          timeout={9000}
          on:close={(e) => { e.preventDefault(); showNotification = false; }}
        />
      {/if}

      <div class="invoice-row">
        <input
          class="amount-input"
          type="number"
          min="0"
          placeholder="Amount (sats)"
          bind:value={amount}
        />
        <Button
          size="field"
          disabled={invDisabled}
          on:click={createInvoice}
        >
          {invoiceLoading ? "Creating…" : "Create Invoice"}
        </Button>
      </div>

      {#if bolt11}
        <div class="qr-section">
          <QrCode size={256} padding={2} value={bolt11} />
          <div class="bolt11-row">
            <span class="bolt11-text">{bolt11}</span>
            <Button size="small" kind="ghost" on:click={() => copyToClipboard(bolt11)}>
              Copy
            </Button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .bot-wrapper {
    font-size: 1rem;
    padding: 0px 25px;
  }
  .bot-container {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    max-width: 600px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .section-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: #8d8d8d;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .balance-value {
    font-size: 1.5rem;
    font-weight: 600;
    color: #f4f4f4;
  }
  .balance-loading {
    height: 2rem;
    display: flex;
    align-items: center;
  }
  .invoice-row {
    display: flex;
    gap: 0.75rem;
    align-items: flex-end;
  }
  .amount-input {
    flex: 1;
    height: 2.5rem;
    padding: 0 0.75rem;
    background: #262626;
    border: 1px solid #525252;
    color: #f4f4f4;
    font-size: 0.875rem;
    outline: none;
  }
  .amount-input:focus {
    border-color: #0f62fe;
  }
  .qr-section {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }
  .bolt11-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
  }
  .bolt11-text {
    font-size: 0.7rem;
    word-break: break-all;
    color: #c6c6c6;
    flex: 1;
  }
</style>
