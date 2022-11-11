<script>
  export let add = () => {};

  import { Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
</script>

<div>
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
</div>

<style>
  .users {
    display: flex;
    align-items: center;
    justify-content: space-between;
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
</style>
