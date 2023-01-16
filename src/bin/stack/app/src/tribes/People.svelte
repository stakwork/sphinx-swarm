<script>
  import { TextInput } from "carbon-components-svelte";
  import { onMount } from "svelte";
  import * as api from "../api";
  import Person from "./Person.svelte";
  import { people } from "../store";
  import VirtualList from "svelte-tiny-virtual-list";

  export let url = "";
  let loading = false;

  let selectedPubkey = "";
  $: filteredUsers = $people;
  $: selectedUser = $people.find((u) => u.owner_pubkey === selectedPubkey);

  let searchTerm = "";

  function filter() {
    if (!searchTerm) return (filteredUsers = $people);
    filteredUsers = $people.filter(
      (u) =>
        u.owner_pubkey.toLowerCase().includes(searchTerm.toLowerCase()) ||
        (u.owner_alias &&
          u.owner_alias.toLowerCase().includes(searchTerm.toLowerCase()))
    );
  }

  async function getUsers() {
    if ($people && $people.length) return;
    loading = true;
    const usersData = await api.tribes.get_people(url);
    people.set(usersData);
    loading = false;
  }

  let topPartElement;
  let heightOfVirtualList = 1000;

  onMount(async () => {
    await getUsers();
    heightOfVirtualList = Math.ceil(window.innerHeight - 315) - 2;
  });

  function formatProps(data) {
    return {
      owner_alias: data.owner_alias,
      owner_pubkey: data.owner_pubkey,
      owner_route_hint: data.owner_route_hint,
      img: data.img,
    };
  }
</script>

<div>
  {#if loading}
    <div class="loading-wrap">
      <h5>Loading People .....</h5>
    </div>
  {:else if selectedUser}
    <Person
      {...formatProps(selectedUser)}
      selected={true}
      select={() => (selectedPubkey = null)}
      {url}
    />
  {:else}
    <div class="people" bind:this={topPartElement}>
      <div class="people-header">
        <p><span class="people-count">{$people.length}</span>People</p>
      </div>
      <section class="sidebar-search-wrap">
        <TextInput
          class="search"
          placeholder="Search by user alias or pubkey"
          bind:value={searchTerm}
          on:input={filter}
        />
      </section>
    </div>

    <VirtualList
      width="100%"
      height={heightOfVirtualList}
      itemCount={filteredUsers.length}
      itemSize={75}
    >
      <div slot="item" let:index let:style {style}>
        <Person
          {...formatProps(filteredUsers[index])}
          select={(pubkey) => (selectedPubkey = pubkey)}
          selected={false}
          {url}
        />
      </div>
    </VirtualList>
  {/if}
</div>

<style>
  .people {
    display: flex;
    flex-direction: column;
  }
  .people-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    margin-top: 1rem;
  }
  .people-header p {
    font-size: 0.9rem;
  }
  .people-count {
    color: rgba(255, 255, 255, 0.5);
    margin-right: 0.5rem;
    font-weight: 700;
  }
</style>
