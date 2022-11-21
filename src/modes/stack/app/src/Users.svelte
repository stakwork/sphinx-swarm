<script>
  export let add = () => {};

  import { Button, TextInput } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { users } from "./store";
  import User from "./User.svelte";

  let selectedPubkey = "";
  $: selectedUser = $users.find((u) => u.pubkey === selectedPubkey);
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
      <TextInput labelText="Search Users" class="users-search" placeholder="Enter user alias, or pubkey" />
    </section>
    {#each $users as user}
      <User
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
