<script lang="ts">
  import {
    Button,
    DataTable,
    Toolbar,
    ToolbarContent,
    ToolbarSearch,
    ToolbarMenu,
    ToolbarMenuItem,
  } from "carbon-components-svelte";
  import UploadIcon from "carbon-icons-svelte/lib/Upload.svelte";
  import { remotes, type Remote } from "./store";

  let selectedRowIds = [];

  function something() {
    console.log("something");
  }

  function remoterow(r: Remote) {
    return { ...r, id: r.root };
  }
</script>

<main>
  <DataTable
    headers={[
      { key: "root", value: "Root Domain" },
      { key: "admin", value: "Admin Name" },
    ]}
    rows={$remotes.map(remoterow)}
    selectable
    bind:selectedRowIds
  >
    <Toolbar>
      <ToolbarContent>
        <ToolbarSearch value="" shouldFilterRows />
        <!-- <ToolbarMenu disabled={false}>
            <ToolbarMenuItem>Restart all</ToolbarMenuItem>
            <ToolbarMenuItem hasDivider>API documentation</ToolbarMenuItem>
            <ToolbarMenuItem hasDivider>Stop all</ToolbarMenuItem>
          </ToolbarMenu> -->
        <Button kind="tertiary" on:click={something} icon={UploadIcon}>
          Do something
        </Button>
      </ToolbarContent>
    </Toolbar>
  </DataTable>
</main>

<style>
  main {
    overflow: auto;
    max-height: var(--body-height);
    width: 100%;
  }
</style>
