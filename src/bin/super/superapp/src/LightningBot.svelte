<script lang="ts">
  import { onMount } from "svelte";
  import { lightningBots } from "./store";
  import LightningBotCard from "./components/LightningBotCard.svelte";
  import { get_lightning_bots_detail } from "../../../../../app/src/api/swarm";
  import { Loading, ToastNotification } from "carbon-components-svelte";
  let error_message = "";
  let loading = true;

  onMount(async () => {
    // get all lightning bots
    try {
      let res = await get_lightning_bots_detail();
      if (res.success) {
        if (res.data) {
          lightningBots.set(res.data);
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
  });
</script>

<main>
  <div class="bot_card_container">
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
    {:else if $lightningBots.length > 0}
      {#each $lightningBots as lightningBot}
        <LightningBotCard {lightningBot} />
      {/each}
    {:else}
      <div class="empty_state_container">
        <p>No Lightning Bot available, Contact Admin to add Lightning bot</p>
      </div>
    {/if}
  </div>
</main>

<style>
  .bot_card_container {
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
