<script>
  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import Tribe from "./Tribe.svelte";
  import { Dropdown } from "carbon-components-svelte";
  import { afterUpdate, onMount } from "svelte";
  import { tribes as tribesApi } from "./api";

  export let add = () => {};
  export let url = "";

  let selectedTribe = "";
  $: selectedTribe = tribes.find((t) => t.name === selectedTribe);
  let tribes = [];

  let selectedId = "0";
  let filterTribes = tribes;

  const filterItems = [
    { id: "0", text: "User count" },
    { id: "1", text: "Recent messages" },
    { id: "2", text: "Previewable" },
  ];

  async function getTribes() {
    const tribesData = await tribesApi.get_tribes(url);
    tribes = tribesData;
  }

  onMount(() => {
    getTribes();
  });

  afterUpdate(() => {
    let filter = filterItems.find((item) => item.id === selectedId);
    const arrayToSort = [...tribes];
    if (filter.text === "User count") {
      filterTribes = arrayToSort.sort(
        (a, b) => b.member_count - a.member_count
      );
    } else if (filter.text === "Previewable") {
      filterTribes = arrayToSort.sort((a, b) => {
        if (b.preview > a.preview) return 1;
        if (b.preview < a.preview) return -1;
        return 0;
      });
    } else if (filter.text === "Recent messages") {
      filterTribes = arrayToSort.sort((a, b) => {
        if (b.last_active > a.last_active) return 1;
        if (b.last_active < a.last_active) return -1;
        return 0;
      });
    } else {
      filterTribes = tribes;
    }
  });

  function formatProps(data) {
    return {
      name: data.name,
      preview: data.preview,
      logo: data.logo,
      price_per_message: data.price_per_message,
      uuid: data.uuid,
      member_count: data.member_count
    }
  }
</script>

<div>
  {#if selectedTribe}
    <Tribe
      {...formatProps(selectedTribe)}
      selected={true}
      select={() => (selectedTribe = null)}
    />
  {:else}
    <div class="divider" />
    <div class="users">
      <p>Current Tribes <span class="users-count">{tribes.length}</span></p>
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
    {#if tribes.length > 0}
      {#each filterTribes as tribe}
        <Tribe
          {...formatProps(tribe)}
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
    margin-top: 15px;
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
