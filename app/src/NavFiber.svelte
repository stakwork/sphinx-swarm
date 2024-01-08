<script>
  import { Button, Tabs, Tab, TabContent } from "carbon-components-svelte";
  import { onMount } from "svelte";
  import SetupAdmin from "./components/NavFiberAdmin.svelte";
  import EnpointPermission from "./components/EnpointPermission.svelte";

  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8001";

  onMount(async () => {});
</script>

<div class="nav-wrapper">
  <Button target="_blank" href={link}>Open Second Brain</Button>
  <div class="tab-container">
    <Tabs>
      <Tab label="Setup Admin"></Tab>
      <Tab label="Endpoint Permissions"></Tab>
      <svelte:fragment slot="content">
        <TabContent>
          <SetupAdmin></SetupAdmin>
        </TabContent>
        <TabContent>
          <EnpointPermission />
        </TabContent>
      </svelte:fragment>
    </Tabs>
  </div>
</div>

<style>
  .nav-wrapper {
    padding: 0px 25px;
  }

  .tab-container {
    margin-top: 1.5rem;
    margin-bottom: 1.5rem;
    width: 100%;
  }
</style>
