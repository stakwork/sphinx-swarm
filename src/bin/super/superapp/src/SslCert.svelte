<script lang="ts">
  import { onMount } from "svelte";
  import { anthropicKeys } from "./store";
  import {
    Button,
    InlineNotification,
    Loading,
    Modal,
    TextInput,
    ToastNotification,
  } from "carbon-components-svelte";
  import KeyDisplayCard from "./components/KeyDisplayCard.svelte";
  import {
    get_ssl_cert_expiry,
    renew_ssl_cert,
    upload_ssl_cert,
  } from "../../../../../app/src/api/swarm";

  let error_message = "";
  let loading = false;
  let expiryDays = 0;
  let openAddAnthropicModal = false;
  let isSubmitting = false;
  let error_notification = false;
  let message;

  onMount(async () => {
    // get all anthropic keys
    await handleGetSslCertExpiryDays();
  });

  async function handleGetSslCertExpiryDays() {
    loading = true;
    try {
      let res = await get_ssl_cert_expiry();
      if (res.success) {
        if (res.data) {
          expiryDays = res.data.day;
        }
      } else {
        error_message = res.message;
      }
    } catch (error) {
      console.log("error: ", error);
      error_message = "Error occured while trying to get ssl cert expiry";
    } finally {
      loading = false;
    }
  }

  async function handleRenewCert() {
    loading = true;
    try {
      const response = await renew_ssl_cert();
      console.log("Renew Ssl Cert response", response);
    } catch (error) {
      console.log("Error renewing certs: ", error);
    } finally {
      loading = false;
    }
  }

  async function handleUploadCert() {
    loading = true;
    try {
      const response = await upload_ssl_cert();
      console.log("Upload Cert to s3 bucker response:", response);
    } catch (error) {
      console.log("Error uploading certs to s3 bucket: ", error);
    } finally {
      loading = false;
    }
  }
</script>

<main>
  <div class="ssl_cert_container">
    {#if loading}
      <Loading />
    {/if}
    <div class="action_button_container">
      <Button on:click={handleRenewCert} disabled={loading}>Renew Cert</Button>
      <Button on:click={handleUploadCert} disabled={loading}
        >Upload Cert to S3</Button
      >
    </div>
    <div class="days_to_expiry">
      <p class="expiry_summary">Cert Expires in:</p>
      <p class="actual_day">
        {error_message
          ? error_message
          : `${expiryDays} day${expiryDays > 1 ? "s" : ""}`}
      </p>
    </div>
  </div>
</main>

<style>
  .ssl_cert_container {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: 1.5rem;
  }

  .days_to_expiry {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  .expiry_summary {
    font-size: 1.3rem;
  }

  .actual_day {
    font-size: 1.5rem;
  }

  .action_button_container {
    display: flex;
    align-items: center;
    gap: 2rem;
  }
</style>
