<script lang="ts">
  import { onMount } from "svelte";
  import { lightningBots } from "./store";
  import LightningBotCard from "./components/LightningBotCard.svelte";
  import { get_lightning_bots_detail } from "../../../../../app/src/api/swarm";

  onMount(async () => {
    // get all lightning bots
    let res = await get_lightning_bots_detail();
    console.log(res);
  });
</script>

<main>
  <div class="bot_card_container">
    {#each $lightningBots as lightningBot}
      <LightningBotCard {lightningBot} />
    {/each}
  </div>
</main>

<style>
  .bot_card_container {
    display: flex;
    flex-direction: column;
    padding: 1.5rem;
  }
</style>
