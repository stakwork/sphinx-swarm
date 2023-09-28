<script lang="ts">
  import { OverflowMenu, OverflowMenuItem } from "carbon-components-svelte";
  import { tribes } from "./store";
  import { splitHost } from "./utils/index";

  export let host = "";

  $: hostTribe = $tribes[splitHost(host)];
</script>

<main>
  <OverflowMenu flipped style="width: auto;">
    <div slot="menu" style="padding: 1rem; color:white;">
      {(hostTribe && hostTribe.length) || 0} Tribes
    </div>
    {#each hostTribe as tribe}
      <OverflowMenuItem>
        <a
          href={`https://tribes.sphinx.chat/t/${tribe.unique_name}`}
          target="_blank"
          class="hyperlink">{tribe.name}</a
        >
      </OverflowMenuItem>
    {/each}
  </OverflowMenu>
</main>

<style>
  .hyperlink {
    text-decoration: none;
    color: white;
    text-transform: capitalize;
  }
</style>
