<script lang="ts">
  import { Button, PasswordInput, Loading, InlineNotification } from "carbon-components-svelte";
  import { get_bot_balance, create_bot_invoice, get_bot_token, get_l402_stats, get_bot_payments } from "./api/swarm";
  import { formatMillisatsToSats, formatSatsNumbers, convertSatsToMilliSats } from "./helpers";
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

  // L402 Stats
  let l402Loading = false;
  let totalL402s: number | string = "—";
  let totalRemainingBalance: number | string = "—";

  // Transactions
  let transactions: any[] = [];
  let txLoading = false;

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

  async function fetchL402Stats() {
    l402Loading = true;
    try {
      const res = await get_l402_stats();
      if (res?.success) {
        totalL402s = res.total_l402s ?? 0;
        totalRemainingBalance = formatSatsNumbers(res.total_remaining_balance ?? 0);
      } else {
        totalL402s = "—";
        totalRemainingBalance = "—";
      }
    } catch (e) {
      totalL402s = "—";
      totalRemainingBalance = "—";
    }
    l402Loading = false;
  }

  async function fetchBotPayments() {
    txLoading = true;
    try {
      const res = await get_bot_payments();
      if (res?.success && res?.data) {
        transactions = Array.isArray(res.data) ? res.data : (res.data.payments ?? []);
      } else {
        transactions = [];
      }
    } catch (e) {
      transactions = [];
    }
    txLoading = false;
  }

  async function refreshAll() {
    await Promise.all([fetchBalance(), fetchL402Stats(), fetchBotPayments()]);
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

  function formatDate(dateStr: string) {
    try {
      return new Date(dateStr).toLocaleString();
    } catch {
      return dateStr;
    }
  }

  onMount(async () => {
    const res = await get_bot_token();
    if (res?.success && res?.data) {
      adminToken = res.data;
    }
    isLoading = false;
    await Promise.all([fetchBalance(), fetchL402Stats(), fetchBotPayments()]);
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

    <!-- L402 Stats Section -->
    <div class="section">
      <div class="section-header">
        <div class="section-title">L402 Stats</div>
        <Button size="small" kind="ghost" on:click={refreshAll}>Refresh All</Button>
      </div>
      {#if l402Loading}
        <div class="balance-loading"><Loading small /></div>
      {:else}
        <div class="stats-grid">
          <div class="stat-item">
            <span class="stat-label">Total L402 Tokens</span>
            <span class="stat-value">{totalL402s}</span>
          </div>
          <div class="stat-item">
            <span class="stat-label">Combined Remaining Balance</span>
            <span class="stat-value">{totalRemainingBalance} sats</span>
          </div>
        </div>
      {/if}
    </div>

    <!-- Transactions Section -->
    <div class="section">
      <div class="section-title">Transactions</div>

      {#if txLoading}
        <div class="balance-loading"><Loading small /></div>
      {:else if transactions.length === 0}
        <div class="empty-state">No payments found.</div>
      {:else}
        <div class="tx-table-wrapper">
          <table class="tx-table">
            <thead>
              <tr>
                <th>Type</th>
                <th>Amount (sats)</th>
                <th>Pubkey</th>
                <th>Date</th>
              </tr>
            </thead>
            <tbody>
              {#each transactions as tx}
                <tr>
                  <td class="type-cell">{tx.type ?? tx.payment_type ?? "—"}</td>
                  <td>{tx.amt_msat != null ? formatMillisatsToSats(tx.amt_msat) : (tx.amount != null ? formatSatsNumbers(tx.amount) : "—")}</td>
                  <td class="pubkey-cell">{tx.pubkey ?? tx.destination ?? "—"}</td>
                  <td>{tx.date ? formatDate(tx.date) : (tx.created_at ? formatDate(tx.created_at) : "—")}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
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
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
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
  .stats-grid {
    display: flex;
    gap: 2rem;
    flex-wrap: wrap;
  }
  .stat-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .stat-label {
    font-size: 0.75rem;
    color: #8d8d8d;
  }
  .stat-value {
    font-size: 1.25rem;
    font-weight: 600;
    color: #f4f4f4;
  }
  .tx-table-wrapper {
    overflow-x: auto;
  }
  .tx-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .tx-table th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    background: #262626;
    color: #8d8d8d;
    font-weight: 600;
    border-bottom: 1px solid #393939;
    white-space: nowrap;
  }
  .tx-table td {
    padding: 0.5rem 0.75rem;
    color: #c6c6c6;
    border-bottom: 1px solid #2d2d2d;
    vertical-align: middle;
  }
  .tx-table tr:hover td {
    background: #262626;
  }
  .pubkey-cell {
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: monospace;
    font-size: 0.75rem;
  }
  .type-cell {
    text-transform: capitalize;
  }
  .empty-state {
    font-size: 0.875rem;
    color: #8d8d8d;
    padding: 1rem 0;
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
