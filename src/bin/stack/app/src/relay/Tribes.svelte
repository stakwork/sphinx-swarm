<script>
  import Delete from "carbon-icons-svelte/lib/RuleCancelled.svelte";
  import { Dropdown, Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";

  let defaultTribes = [
    {
      id: 1,
      name: "Testing Sphinx",
    },
    {
      id: 2,
      name: "Sphinx dev",
    },
    {
      id: 3,
      name: "Nostr",
    },
  ];

  $: tribe = "";

  function deleteTribe(id) {
    const tribeIndex = defaultTribes.findIndex((t) => t.id === id);

    if (tribeIndex !== -1) {
      defaultTribes = defaultTribes.filter((_, i) => i !== tribeIndex);
    }
  }

  function addDefaulttribe() {}
</script>

<div class="tribes-wrap">
  <section class="header-wrap">
    <h1 class="default-header">Default Tribes</h1>
    <small>(A maximum of 5 default tribes)</small>
  </section>
  <div class="divider" />

  <div class="tribes-data">
    {#each defaultTribes as tribe}
      <section class="tribes">
        <p class="name">{tribe.name}</p>
        <button on:click={() => deleteTribe(tribe.id)} class="delete-btn"
          ><Delete size={20} /></button
        >
      </section>
    {/each}

    <div class="divider" />
    <section class="add-tribe-wrap">
      <label for="tribes">Add tribe</label>
      <section class="form">
        <Dropdown bind:selectedId={tribe} />
        <div class="spacer" />
        {#if tribe}
          <Button on:click={addDefaulttribe} size="field" icon={Add}>Add</Button
          >
        {/if}
      </section>
    </section>
  </div>
</div>

<style>
  .tribes-wrap {
    padding: 1.5rem;
  }
  .header-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
  }

  .header-wrap .default-header {
    font-size: 1rem;
    font-weight: 400;
  }
  .header-wrap small {
    font-size: 0.65rem;
    color: #c6c6c6;
    margin-left: 15px;
  }
  .tribes-data .tribes {
    display: flex;
    flex-direction: row;
    align-items: center;
    padding: 0.5rem 0rem;
  }
  .tribes-data .tribes .name {
    font-size: 0.9rem;
    padding: 0;
    margin: 0;
  }
  .tribes-data .tribes .delete-btn {
    margin: 0;
    margin-left: auto;
    background-color: transparent;
    color: red;
    padding: 0;
    border: 0;
    width: 20px;
    height: 20px;
  }
  .add-tribe-wrap {
    margin-top: 20px;
  }
  .add-tribe-wrap label {
    font-size: 0.8rem;
    margin-bottom: 15px;
    display: block;
    color: #c6c6c6;
  }
  .add-tribe-wrap .form {
    text-align: center;
  }
</style>
