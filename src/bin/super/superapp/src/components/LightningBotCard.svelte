<script lang="ts">
  import type { ILightningBot } from "../types/types";
  import { onMount } from "svelte";

  export let lightningBot: ILightningBot;
  onMount(() => {
    // console.log("tester");
  });
</script>

<div class="bot_card_container"></div>

<style>
  .bot_card_container {
    border-radius: 1rem;
    height: 10rem;
    background-color: tomato;
    margin-bottom: 1rem;
  }
</style>
