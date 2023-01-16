<script lang="ts">
  import Tribe from "./Tribe.svelte";
  import { Dropdown, TextInput } from "carbon-components-svelte";
  import { onMount } from "svelte";
  import * as api from "../api";
  import { tribes } from "../store";
  import VirtualList from "svelte-tiny-virtual-list";
  import InfiniteLoading from "svelte-infinite-loading";
  import _ from "lodash";

  export let url = "";

  let topPartElement;

  let loading = false;

  let searchTerm = "";

  let page = $tribes.page;

  let limit = 75;

  let selectedTribe;
  $: selectedTribe = $tribes.data.find((t) => t.uuid === selectedTribe);

  let selectedId = "0";
  let filterTribes = $tribes.data;

  const filterItems = [
    { id: "0", text: "User count" },
    { id: "1", text: "Recent messages" },
    { id: "2", text: "Previewable" },
  ];

  async function search() {
    const debounced = _.debounce(
      async () => {
        if (!searchTerm) return (filterTribes = $tribes.data);
        filterTribes = await api.tribes.get_tribes(
          url,
          "",
          searchTerm.toLocaleLowerCase()
        );
      },
      0,
      {}
    );
    debounced();
  }

  let heightOfVirtualList = 0;

  async function getTotalTribes() {
    const total = await api.tribes.get_tribes_total(url);

    if($tribes.total !== total) {
      tribes.set({
        total,
        data: $tribes.data,
        page,
      })
    }
  }

  onMount(async () => {
    getTotalTribes();
    sort();

    const rect = topPartElement.getBoundingClientRect();
    heightOfVirtualList = Math.ceil(window.innerHeight - rect.bottom) - 58 - 2;
  });

  function sort() {
    let filter = filterItems.find((item) => item.id === selectedId);
    const arrayToSort = [...$tribes.data];

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
      filterTribes = $tribes.data;
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

  async function infiniteHandler({ detail: { loaded, complete } }) {
    const tribesData = await api.tribes.get_tribes(url, "", "", page, limit);

    if (tribesData.length) {
      page += 1;
      filterTribes = [...filterTribes, ...tribesData];

      // save data to store
      tribes.set(
        {
          page,
          data: filterTribes,
          total: $tribes.total
        });

      loaded();
    } else {
      complete();
    }
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
      <p><span class="tribes-count">{$tribes.data.length}</span>Tribes</p>
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

    <section class="sidebar-search-wrap">
      <form on:submit|preventDefault={search}>
        <TextInput
          class="search"
          placeholder="Search by tribe name"
          bind:value={searchTerm}
        />
      </form>
    </section>

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
      <div slot="footer">
        <InfiniteLoading on:infinite={infiniteHandler} />
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
