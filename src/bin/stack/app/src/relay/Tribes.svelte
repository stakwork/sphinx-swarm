<script>
  import { Dropdown, Button } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import {allTribes} from "../store";

  let items = $allTribes.map((t) => ({
    id: t.uuid,
    text: t.name,
  }));

  items = [{id: "", text: "Select a tribe"}, ...items];

  let defaultTribes = [
    {
      id: "1",
      name: "Testing Sphinx",
    },
    {
      id: "2",
      name: "Sphinx dev",
    },
    {
      id: "3",
      name: "Nostr",
    },
  ];

  let selectedTribe = "";

  function deleteTribe(id) {
    const tribeIndex = defaultTribes.findIndex((t) => t.id === id);

    if (tribeIndex !== -1) {
      defaultTribes = defaultTribes.filter((_, i) => i !== tribeIndex);
    }
  }

  function addDefaulttribe(id) {
    // Check if tribe as already been added
    const tribeIndex = defaultTribes.findIndex(t => t.id === id);

    if(defaultTribes.length < 5 && tribeIndex === -1) {
      const tribe = items.find(t => t.id === id);

      defaultTribes = [...defaultTribes, {id: tribe.id, name: tribe.text}]
    }
  }
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
          >X</button
        >
      </section>
    {/each}

    <div class="divider" />
    <section class="add-tribe-wrap">
      <label for="tribes">Add tribe</label>
      <section class="form">
        <Dropdown bind:selectedId={selectedTribe} value="" {items} />
        <div class="spacer" />
        {#if selectedTribe && defaultTribes.length < 5}
          <Button on:click={() => addDefaulttribe(selectedTribe)} size="field" icon={Add}>Add</Button
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
    font-size: 0.95rem;
    font-weight: bolder;
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
