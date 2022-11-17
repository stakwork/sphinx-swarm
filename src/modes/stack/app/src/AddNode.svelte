<script lang="ts">
  import {
    Button,
    Modal,
    MultiSelect,
    Dropdown,
    TextInput,
  } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { controls } from "./controls";
  import { afterUpdate, beforeUpdate } from "svelte";

  let open = false;
  let nameValue = "";
  let ctrls = controls["NodeTypes"];
  $: selectedId = ctrls[0].items[0].id;
  let selectConnections = [];
  let items = ctrls[0].items;

  const availableConnections = {
    Btc: ["****"],
    Lnd: ["Btc"],
    Proxy: ["Lnd"],
    Relay: ["Lnd", "Proxy", "Meme", "Tribes"],
  };

  function getType(selectedId: string): string {
    const item = items.find(node => node.id === selectedId);
    return item.text;
  }


  $: connections = availableConnections[getType(selectedId)];

  afterUpdate(() => {
    connections = availableConnections[getType(selectedId)];
    console.log("Node Store After ===", selectedId, connections);
  });

  $: linkItems = connections && connections.map((c, i) =>({
    id: `conn${i}`,
    text: c
  }))

</script>

<section class="add-node-btn">
  <Button type="submit" size="field" icon={Add} on:click={() => (open = true)}
    >Add New Node</Button
  >

  <Modal
    bind:open
    modalHeading="Add Node"
    hasForm={true}
    class="new-node-modal"
    size="sm"
    primaryButtonText="Add"
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (open = !open)}
  >
    <section class="modal-content">
      <Dropdown titleText={"Node type"} bind:selectedId {items} />
      <div class="spacer" />
      <TextInput
        labelText={"Name"}
        placeholder={"Enter node name"}
        bind:value={nameValue}
      />
      <div class="spacer" />
      <MultiSelect
        bind:selectedIds={selectConnections}
        titleText="Connections"
        items={linkItems}
        label="Select node connections"
      />
      <!-- {:else if type === "dropdown" && items && usefor === "nodeconnections"}
  <Dropdown titleText={name} bind:selectedId items={getConnections(selectedId)} /> -->
    </section>
  </Modal>
</section>

<style>
  .add-node-btn {
    margin-left: auto;
    margin-right: 2.5rem;
  }

  .modal-content {
    padding: 0px 1.5rem;
  }
</style>
