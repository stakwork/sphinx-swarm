<script lang="ts">
  import {
    Button,
    InlineNotification,
    Modal,
    TextInput,
    ToastNotification,
  } from "carbon-components-svelte";
  import QrCode from "svelte-qrcode";
  import {
    convertMillisatsToSats,
    convertSatsToMilliSats,
    formatSatsNumbers,
  } from "../../../../../../app/src/helpers";
  import { splitPubkey } from "../../../../../../app/src/helpers/swarm";
  import type { ILightningBot } from "../types/types";
  import {
    change_lightning_bot_label,
    create_invoice_for_lightning_bot,
  } from "../../../../../../app/src/api/swarm";
  import { fectAndRefreshLightningBotDetails } from "../utils";
  import { lightningBots } from "../store";
  export let lightningBot: ILightningBot;

  let open_change_label = false;
  let isSubmitting = false;
  let new_label = "";
  let error_notification = false;
  let message = "";
  let show_notification = false;
  let open_create_invoice = false;
  let amount = 0;
  let show_invoice_notification = false;
  let invoice_success = false;
  let invoice = "";

  function handleOpenChangeBotLabelModal() {
    open_change_label = true;
    new_label = lightningBot.label;
  }

  function handleOnCloseChangeBotLabel() {
    open_change_label = false;
    new_label = "";
  }

  async function handleChangeBotLabel() {
    isSubmitting = true;
    try {
      let res = await change_lightning_bot_label({
        id: lightningBot.id,
        new_label,
      });
      message = res.message;
      if (res.success) {
        const refresh = await fectAndRefreshLightningBotDetails(lightningBots);
        if (!refresh.success) {
          message = refresh.message;
          error_notification = true;
          return;
        }
        show_notification = true;
        open_change_label = false;
        new_label = "";
      } else {
        error_notification = true;
      }
    } catch (error) {
      console.log("Error trying to change label: ", error);
      message = "Error trying to change label";
      error_notification = true;
    } finally {
      isSubmitting = false;
    }
  }

  function handleOpenCreateInvoice() {
    open_create_invoice = true;
    amount = 0;
  }

  async function handleCreateInvoice() {
    isSubmitting = true;
    try {
      //convert amount to number, convert to milisats and send to the backend
      const res = await create_invoice_for_lightning_bot({
        id: lightningBot.id,
        amt_msat: convertSatsToMilliSats(amount),
      });
      message = res.message;
      if (res.success !== true) {
        return;
      }
      invoice = res.data.bolt11;
      invoice_success = true;
    } catch (error) {
      console.log("Error creating Invoice: ", error);
      message = "Error creating invoice, please contact admin";
    } finally {
      isSubmitting = false;
      show_invoice_notification = true;
    }
  }

  function copyToClipboard(value) {
    navigator.clipboard.writeText(value);
  }

  function handleOnCloseCreateInvoice() {
    invoice = "";
    amount = 0;
    open_create_invoice = false;
  }
</script>

<div class="bot_card_container">
  {#if lightningBot.error_message}
    <div class="success_toast_container">
      <p>Label: <span class="card_value">{lightningBot.label}</span></p>
      <ToastNotification
        lowContrast
        kind={"error"}
        title={"Error"}
        subtitle={lightningBot.error_message}
        fullWidth={true}
      />
    </div>{:else}
    <div class="bot_card">
      {#if show_notification}
        <div class="success_toast_container">
          <ToastNotification
            lowContrast
            kind={"success"}
            title={"Success"}
            subtitle={message}
            timeout={3000}
            on:close={(e) => {
              e.preventDefault();
              show_notification = false;
            }}
            fullWidth={true}
          />
        </div>
      {/if}
      <p>Label: <span class="card_value">{lightningBot.label}</span></p>
      <p>
        Public Key: <span class="card_value"
          >{splitPubkey(lightningBot.contact_info)}</span
        >
      </p>
      <p>
        Balance : <span class="card_value"
          >{formatSatsNumbers(
            convertMillisatsToSats(lightningBot.balance_in_msat)
          )}
          Sats</span
        >
      </p>
      <p>
        Network : <span class="card_value">{lightningBot.network} </span>
      </p>
      <p>
        Alias : <span class="card_value">{lightningBot.alias} </span>
      </p>
      <div class="action_container">
        <Button on:click={() => handleOpenChangeBotLabelModal()}
          >Change Bot Label</Button
        >

        <Button on:click={() => handleOpenCreateInvoice()}
          >Create Invoice</Button
        >
      </div>
    </div>
  {/if}

  <Modal
    bind:open={open_change_label}
    modalHeading="Change Bot Label"
    primaryButtonDisabled={lightningBot.label === new_label || isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Update"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open_change_label = false)}
    on:open
    on:close={handleOnCloseChangeBotLabel}
    on:submit={handleChangeBotLabel}
  >
    {#if error_notification}
      <InlineNotification
        kind="error"
        title="Error:"
        subtitle={message}
        timeout={8000}
        on:close={(e) => {
          e.preventDefault();
          error_notification = false;
        }}
      />
    {/if}
    <div class="input_container">
      <TextInput
        labelText="Current Password"
        placeholder="Enter Current Password..."
        bind:value={new_label}
      />
    </div>
  </Modal>

  <Modal
    bind:open={open_create_invoice}
    modalHeading="Create Invoice"
    primaryButtonDisabled={amount === 0 || !amount || isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Create"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open_create_invoice = false)}
    on:open
    on:close={handleOnCloseCreateInvoice}
    on:submit={handleCreateInvoice}
  >
    {#if show_invoice_notification}
      <InlineNotification
        lowContrast
        kind={invoice_success ? "success" : "error"}
        title={invoice_success ? "Success:" : "Error:"}
        subtitle={message}
        timeout={9000}
        on:close={(e) => {
          e.preventDefault();
          show_invoice_notification = false;
        }}
      />
    {/if}
    <div class="input_container">
      <TextInput
        labelText="Amount (satoshis)"
        placeholder="Enter Invoice Amount in Sats..."
        type="number"
        bind:value={amount}
      />
    </div>
    {#if invoice}
      <section class="invoice-data">
        <div class="qr_container">
          <p class="invoice-title">Invoice QR code</p>
          <QrCode size={256} padding={22} value={invoice} />
        </div>

        <div class="invoice">
          {invoice}
        </div>

        <Button
          kind="tertiary"
          class="invoice-btn"
          on:click={() => copyToClipboard(invoice)}>Copy Invoice</Button
        >
      </section>
    {/if}
  </Modal>
</div>

<style>
  .bot_card_container {
    border-radius: 1rem;
    min-height: 10rem;
    margin-bottom: 1rem;
    display: flex;
    padding: 1.5rem;
    flex-direction: column;
    border: 1px solid #f7e2e2;
  }

  .card_value {
    font-size: 1.5rem;
  }

  .bot_card {
    display: flex;
    flex-direction: column;
  }

  .action_container {
    display: flex;
    gap: 2rem;
    flex-wrap: wrap;
    margin-top: 2rem;
  }

  .qr_container {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: center;
    flex-direction: column;
  }

  .invoice-title {
    text-align: center;
    font-size: 1rem;
    margin-bottom: 0.7rem;
    margin-top: 0.7rem;
    padding-right: 0;
  }

  .invoice-data {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
  }

  .invoice {
    overflow: scroll;
    text-overflow: clip;
    height: 110px;
    overflow-wrap: break-word;
    font-size: 0.9rem;
    border: 0.5px solid #fff;
    min-width: 100%;
    max-width: 100%;
    border-radius: 10px;
    margin-top: 20px;
    padding: 10px;
    margin-bottom: 0.7rem;
  }
</style>
