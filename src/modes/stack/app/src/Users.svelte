<script>
  export let add = () => {};

  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
</script>

<div class="users">
  <p>42 Users</p>
  <Button on:click={add} type="submit" icon={Add} disabled={false}
    >Add User</Button
  >
</div>

<style>
  .users {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
</style>
