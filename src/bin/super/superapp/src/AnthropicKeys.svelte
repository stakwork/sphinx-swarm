<script lang="ts">
  import { onMount } from "svelte";
  import { anthropicKeys } from "./store";
  import { Loading, ToastNotification } from "carbon-components-svelte";
  import KeyDisplayCard from "./components/KeyDisplayCard.svelte";
  import { get_anthropic_keys } from "../../../../../app/src/api/swarm";

  let error_message = "";
  let loading = false;

  onMount(async () => {
    // get all lightning bots
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
      error_message =
        "Error occured while trying to get lightning bots details";
    } finally {
      loading = false;
    }
  }
</script>

<main>
  <div class="keys_card_container">
    {#if loading}
      <Loading />
    {/if}
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
