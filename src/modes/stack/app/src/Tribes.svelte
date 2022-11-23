<script>
  export let add = () => {};

  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { tribes } from "./store";
  import Tribe from "./Tribe.svelte";
  import { Dropdown } from "carbon-components-svelte";

  let selectedTribe = "";
  $: selectedTribe = $tribes.find((t) => t.name === selectedTribe);
</script>

<div>
  {#if selectedTribe}
    <Tribe
      {...selectedTribe}
      selected={true}
      select={() => (selectedTribe = null)}
    />
  {:else}
    <div class="divider" />
    <div class="users">
      <p>Current Tribes <span class="users-count">42</span></p>
      <Button
        on:click={add}
        kind="tertiary"
        type="submit"
        size="field"
        icon={Add}
        disabled={false}>Add Tribe</Button
      >
    </div>
    <div class="divider" />
    <section class="filter-wrap">
      <aside>
        <Dropdown
          type="inline"
          titleText="Filter tribes by: "
          selectedId="0"
          items={[
            { id: "0", text: "User count" },
            { id: "1", text: "Recent messages" },
            { id: "2", text: "Previewable" },
          ]}
        />
      </aside>
    </section>
    {#each $tribes as tribe}
      <Tribe
        {...tribe}
        select={(name) => (selectedTribe = name)}
        selected={false}
      />
    {/each}
  {/if}
</div>

<style>
  .users {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
  }
  .users p {
    font-size: 0.9rem;
  }
  .users-count {
    color: rgba(255, 255, 255, 0.5);
    margin-left: 15px;
    font-weight: 700;
  }
  .divider {
    min-height: 2px;
    background: #101317;
    display: block;
    width: 100%;
    margin: 15px 0px;
  }
  .filter-wrap {
    display: flex;
    padding: 0 1.3rem;
  }
  .filter-wrap aside {
    margin-left: auto;
  }
</style>
