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
    add_anthropic_key,
    get_anthropic_keys,
  } from "../../../../../app/src/api/swarm";

  let error_message = "";
  let loading = false;
  let anthropicKey = "";
  let openAddAnthropicModal = false;
  let isSubmitting = false;
  let error_notification = false;
  let message;

  onMount(async () => {
    // get all anthropic keys
    await handleGetAnthropicKeys();
  });

  async function handleGetAnthropicKeys() {
    loading = true;
    try {
      let res = await get_anthropic_keys();
      if (res.success) {
        if (res.data) {
          anthropicKeys.set(res.data);
        }
      } else {
        error_message = res.message;
      }
    } catch (error) {
      console.log("error: ", error);
      error_message = "Error occured while trying to get anthropic keys";
    } finally {
      loading = false;
    }
  }

  async function handleAddAnthropicKey() {
    try {
      isSubmitting = true;
      if (!anthropicKey.trim()) {
        message = "Please provide a valid anthropic key";
        error_notification = true;
        return;
      }
      let res = await add_anthropic_key({ key: anthropicKey.trim() });
      console.log("res:", res);
      if (res.success) {
        // close modal
        handleClosenAddAnthropicKeyModal();
        // get recent keys
        handleGetAnthropicKeys();
      } else {
        message = res.message;
      }
    } catch (error) {
      console.log("error: ", error);
      message = "Error occured while trying to add anthropic keys";
    } finally {
      isSubmitting = false;
    }
  }

  function handleOpenAddAnthropicKeyModal() {
    openAddAnthropicModal = true;
  }

  function handleClosenAddAnthropicKeyModal() {
    anthropicKey = "";
    openAddAnthropicModal = false;
  }
</script>

<main>
  <div class="keys_card_container">
    {#if loading}
      <Loading />
    {/if}
    <div class="add_key_container">
      <Button on:click={handleOpenAddAnthropicKeyModal}
        >Add Anthropic Key</Button
      >
    </div>
    {#if error_message}
      <div class="success_toast_container">
        <ToastNotification
          lowContrast
          kind={"error"}
          title={"Error"}
          subtitle={error_message}
          fullWidth={true}
        />
      </div>
    {:else if $anthropicKeys.length > 0}
      {#each $anthropicKeys as key}
        <KeyDisplayCard value={key} />
      {/each}
    {:else}
      <div class="empty_state_container">
        <p>No Anthropic keys available at the moment</p>
      </div>
    {/if}
  </div>

  <Modal
    bind:open={openAddAnthropicModal}
    modalHeading="Add Anthropic Key"
    primaryButtonDisabled={!anthropicKey || isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Add"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={handleClosenAddAnthropicKeyModal}
    on:open
    on:close={handleClosenAddAnthropicKeyModal}
    on:submit={handleAddAnthropicKey}
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
        labelText="Anthropic Key"
        placeholder="Enter Anthropic Key..."
        bind:value={anthropicKey}
      />
    </div>
  </Modal>
</main>

<style>
  .keys_card_container {
    display: flex;
    flex-direction: column;
    padding: 1.5rem;
  }

  .success_toast_container {
    margin-bottom: 2rem;
  }

  .empty_state_container {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .empty_state_container p {
    font-size: 1.5rem;
  }
</style>
