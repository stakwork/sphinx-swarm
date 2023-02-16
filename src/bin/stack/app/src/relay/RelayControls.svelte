<script>
  import Users from "./Users.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import Admin from "./Admin.svelte";

  export let tag = "";
</script>

<Tabs>
  <Tab label="Users" />
  <Tab label="Configuration" />
  <svelte:fragment slot="content">
    <TabContent><Users {tag} /></TabContent>
    <TabContent>
      <Admin {tag} />
    </TabContent>
  </svelte:fragment>
</Tabs>
