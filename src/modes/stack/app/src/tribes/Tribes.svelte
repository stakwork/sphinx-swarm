<script lang="ts">
  import Tribe from "./Tribe.svelte";
  import { Dropdown } from "carbon-components-svelte";
  import { onMount } from "svelte";
  import * as api from "../api";
  import { tribes } from "../store";
  import VirtualList from "svelte-tiny-virtual-list";

  export let url = "";

  let topPartElement;

  let loading = false;

  let selectedTribe;
  $: selectedTribe = $tribes.find((t) => t.uuid === selectedTribe);

  let selectedId = "0";
  let filterTribes = $tribes;

  const filterItems = [
    { id: "0", text: "User count" },
    { id: "1", text: "Recent messages" },
    { id: "2", text: "Previewable" },
  ];

  async function getTribes() {
    if ($tribes && $tribes.length) return;
    loading = true;
    const tribesData = await api.tribes.get_tribes(url);
    tribes.set(tribesData);
    loading = false;
  }

  let heightOfVirtualList = 1000;

  onMount(async () => {
    await getTribes();
    sort();
    const rect = topPartElement.getBoundingClientRect();
    heightOfVirtualList = Math.ceil(window.innerHeight - rect.bottom) - 2;
  });

  function sort() {
    let filter = filterItems.find((item) => item.id === selectedId);
    const arrayToSort = [...$tribes];
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
      filterTribes = $tribes;
    }
  }

  function formatProps(data) {
    return {
      name: data.name,
      preview: data.preview,
      img: data.img,
      price_per_message: data.price_per_message,
      uuid: data.uuid,
      member_count: data.member_count,
      unique_name: data.unique_name,
    };
  }
</script>

<div>
  {#if loading}
    <div class="loading-wrap">
      <h5>Loading Tribes .....</h5>
    </div>
  {:else if selectedTribe}
    <Tribe
      {...formatProps(selectedTribe)}
      selected={true}
      select={() => (selectedTribe = null)}
      {url}
    />
  {:else}
    <div class="tribes" bind:this={topPartElement}>
      <p><span class="tribes-count">{$tribes.length}</span>Tribes</p>
      <section class="filter-wrap">
        <aside>
          <Dropdown
            type="inline"
            titleText="Filter:"
            bind:selectedId
            items={filterItems}
            on:select={sort}
          />
        </aside>
      </section>
    </div>
    <VirtualList
      width="100%"
      height={heightOfVirtualList}
      itemCount={filterTribes.length}
      itemSize={75}
    >
      <div slot="item" let:index let:style {style}>
        <Tribe
          {...formatProps(filterTribes[index])}
          select={(uuid) => (selectedTribe = uuid)}
          selected={false}
          {url}
        />
      </div>
    </VirtualList>
  {/if}
</div>

<style>
  .tribes {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    margin-top: 1rem;
  }
  .tribes p {
    font-size: 0.9rem;
  }
  .tribes-count {
    color: rgba(255, 255, 255, 0.5);
    margin-right: 0.5rem;
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
