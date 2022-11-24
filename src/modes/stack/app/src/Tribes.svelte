<script>
  export let add = () => {};

  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { tribes } from "./store";
  import Tribe from "./Tribe.svelte";
  import { Dropdown } from "carbon-components-svelte";
  import { afterUpdate } from "svelte";

  let selectedTribe = "";
  $: selectedTribe = $tribes.find((t) => t.name === selectedTribe);
  let selectedId = "0";
  let filterTribes = [];

  const filterItems = [
    { id: "0", text: "User count" },
    { id: "1", text: "Recent messages" },
    { id: "2", text: "Previewable" },
  ];

  afterUpdate(() => {
    let filter = filterItems.find((item) => item.id === selectedId);

    if (filter.text === "User count") {
      filterTribes = $tribes.sort((a, b) => b.userCount - a.userCount);
    } else if (filter.text === "Previewable") {
      filterTribes = $tribes.sort((a, b) => {
        if (b.preview > a.preview) return 1;
        if (b.preview < a.preview) return -1;
        return 0;
      });
    } else {
      filterTribes = [];
    }
  });
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
          bind:selectedId
          items={filterItems}
        />
      </aside>
    </section>
    {#if filterTribes.length > 0}
      {#each filterTribes as tribe}
        <Tribe
          {...tribe}
          select={(name) => (selectedTribe = name)}
          selected={false}
        />
      {/each}
    {:else}
      {#each $tribes as tribe}
        <Tribe
          {...tribe}
          select={(name) => (selectedTribe = name)}
          selected={false}
        />
      {/each}
    {/if}
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
  .filter-wrap {
    display: flex;
    padding: 0 1.3rem;
  }
  .filter-wrap aside {
    margin-left: auto;
  }
</style>
