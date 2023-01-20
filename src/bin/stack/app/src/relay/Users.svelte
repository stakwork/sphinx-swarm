<script lang="ts">
  export let add = () => {};

  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { users } from "../store";
  import User from "./User.svelte";
  import { afterUpdate } from "svelte";
  import AddUser from "./AddUser.svelte";

  let selectedPubkey = "";
  $: filteredUsers = $users;
  $: selectedUser = $users.find((u) => u.pubkey === selectedPubkey);

  let searchTerm = "";

  afterUpdate(() => {
    if (!searchTerm) return (filteredUsers = $users);
    filteredUsers = $users.filter(
      (u) =>
        u.pubkey.toLowerCase().includes(searchTerm.toLowerCase()) ||
        (u.alias && u.alias.toLowerCase().includes(searchTerm.toLowerCase()))
    );
  });

  type UserPage = "main" | "add_user";
  let page: UserPage = "main";

  function toggleAddUser() {
    if (page === "add_user") {
      page = "main";
    } else {
      page = "add_user";
    }
  }
</script>

<div>
  {#if selectedUser}
    <User
      {...selectedUser}
      selected={true}
      select={() => (selectedPubkey = null)}
    />
  {:else}
    <div class="divider" />
    <div class="users">
      <p>Current Users <span class="users-count">42</span></p>
      <Button
        on:click={toggleAddUser}
        kind="tertiary"
        type="submit"
        size="field"
        icon={Add}
        disabled={false}>Add User</Button
      >
    </div>
    <div class="divider" />
    {#if page === "add_user"}
      <AddUser back={toggleAddUser} />
    {:else if page === "main"}
      <section class="search-wrap">
        <TextInput
          labelText="Search Users"
          class="users-search"
          placeholder="Search by user alias or pubkey"
          bind:value={searchTerm}
        />
      </section>
      {#each filteredUsers as user}
        <User
          {...user}
          select={(pubkey) => (selectedPubkey = pubkey)}
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

  .search-wrap {
    margin: 0 1rem;
    margin-bottom: 10px;
  }
</style>
