<script lang="ts">
  import {
    Button,
    Modal,
    MultiSelect,
    Dropdown,
    TextInput,
  } from "carbon-components-svelte";
  import Add from "carbon-icons-svelte/lib/Add.svelte";
  import { NodeType, Node, allNodeTypes } from "./nodes";
  import { stack } from "./store";

  let open = false;
  let name = "";
  let type: NodeType = "Btc";
  let links = [];

  let typeItems = allNodeTypes.map((nt) => ({
    id: nt,
    text: nt,
  }));

  const availableConnections: { [k: string]: NodeType[] } = {
    Lnd: ["Btc"],
    Proxy: ["Lnd"],
    Relay: ["Lnd", "Proxy", "Meme", "Tribes"],
  };

  $: connections = availableConnections[type];
  $: linkItems =
    connections &&
    connections.map((c) => ({
      id: c,
      text: c,
    }));

  function add() {
    const newNode: Node = {
      name,
      type,
      links,
      place: "Internal",
    };
    return console.log(newNode);
    // stack.update((s) => ({
    //   network: s.network,
    //   nodes: [...s.nodes, newNode],
    // }));
  }
  function typeSelected() {
    // reset the state
    links = [];
    name = "";
  }

  $: ok = name && type;
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
    on:submit={add}
    primaryButtonDisabled={!ok}
  >
    <section class="modal-content">
      <Dropdown
        titleText="Node type"
        bind:selectedId={type}
        items={typeItems}
        on:select={typeSelected}
      />
      <div class="spacer" />
      <TextInput
        labelText={"Name"}
        placeholder={"Enter node name"}
        bind:value={name}
      />
      <div class="spacer" />
      <MultiSelect
        direction="top"
        titleText="Connections"
        label="Select node connections"
        bind:selectedIds={links}
        items={linkItems}
      />
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
  .spacer {
    height: 1rem;
  }
</style>
