<script lang="ts">
  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { users } from "../store";
  import User from "./User.svelte";
  import { afterUpdate } from "svelte";
  import AddUser from "./AddUser.svelte";
  import { onMount } from "svelte";
  import * as api from "../api";

  export let tag = "";

  let selectedPubkey = "";
  $: filteredUsers = normalUsers($users);
  $: selectedUser = normalUsers($users).find(
    (u) => u.public_key === selectedPubkey
  );

  let searchTerm = "";

  async function getUsers() {
    const userList = await api.relay.list_users(tag);
    users.set(userList.users);
  }
  onMount(async () => {
    getUsers();
  });

  export function normalUsers(us) {
    return us.filter((u) => !u.is_admin && !u.deleted);
  }

  afterUpdate(() => {
    if (!searchTerm) return (filteredUsers = normalUsers($users));
    filteredUsers = normalUsers($users).filter(
      (u) =>
        u.public_key.toLowerCase().includes(searchTerm.toLowerCase()) ||
        (u.alias && u.alias.toLowerCase().includes(searchTerm.toLowerCase()))
    );
  });

  type UserPage = "main" | "add_user";
  let page: UserPage = "main";

  async function backToMain() {
    await getUsers();
    page = "main";
  }
  function toAddUser() {
    page = "add_user";
  }
</script>

<div>
  {#if selectedUser}
    <User
      user={selectedUser}
      selected={true}
      select={() => (selectedPubkey = null)}
    />
  {:else}
    <div class="divider" />
    <div class="users">
      <p>
        Users
        <span class="users-count">
          {filteredUsers.length}
        </span>
      </p>
      <Button
        on:click={toAddUser}
        kind="tertiary"
        type="submit"
        size="field"
        icon={Add}
        disabled={false}>User</Button
      >
    </div>
    <div class="divider" />
    {#if page === "add_user"}
      <AddUser back={backToMain} {tag} />
    {:else if page === "main"}
      {#if filteredUsers.length}
        <section class="search-wrap">
          <TextInput
            labelText="Search Users"
            class="users-search"
            placeholder="Search by user alias or pubkey"
            bind:value={searchTerm}
          />
        </section>
      {/if}
      {#each filteredUsers as user}
        <User
          {user}
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
