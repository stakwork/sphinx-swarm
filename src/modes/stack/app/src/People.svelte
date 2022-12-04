<script>
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import People from "./People.svelte";
  import { afterUpdate, onMount } from "svelte";
  import { tribes as tribesApi } from "./api";
  import Person from "./Person.svelte";

  export let add = () => {};
  export let url = "";

  let users = [];
  let selectedPubkey = "";
  $: filteredUsers = users;
  $: selectedUser = users.find((u) => u.owner_pubkey === selectedPubkey);

  let searchTerm = "";

  afterUpdate(() => {
    if (!searchTerm) return (filteredUsers = users);
    filteredUsers = users.filter(
      (u) =>
        u.owner_pubkey.toLowerCase().includes(searchTerm.toLowerCase()) ||
        (u.owner_alias && u.owner_alias.toLowerCase().includes(searchTerm.toLowerCase()))
    );
  });

  async function getUsers() {
    const usersData = await tribesApi.get_people(url);
    console.log("Users ====", usersData);
    users = usersData;
  }

  onMount(() => {
    getUsers();
  });
</script>

<div>
  {#if selectedUser}
    <Person
      {...selectedUser}
      selected={true}
      select={() => (selectedPubkey = null)}
    />
  {:else}
    <div class="divider" />
    <div class="users">
      <p>Current Users <span class="users-count">{users.length}</span></p>
      <Button
        on:click={add}
        kind="tertiary"
        type="submit"
        size="field"
        icon={Add}
        disabled={false}>Add User</Button
      >
    </div>
    <div class="divider" />
    <section class="search-wrap">
      <TextInput
        labelText="Search Users"
        class="users-search"
        placeholder="Search by user alias or pubkey"
        bind:value={searchTerm}
      />
    </section>
    {#each filteredUsers as user}
      <Person
        {...user}
        select={(pubkey) => (selectedPubkey = pubkey)}
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

  .search-wrap {
    margin: 0 1rem;
    margin-bottom: 10px;
  }
</style>