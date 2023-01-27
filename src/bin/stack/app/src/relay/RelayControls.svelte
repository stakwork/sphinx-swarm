<script>
  import Users from "./Users.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import DefaultTribes from "./DefaultTribes.svelte";

  export let tag = "";
</script>

<Tabs>
  <Tab label="Users" />
  <Tab label="Configuration" />
  <svelte:fragment slot="content">
    <TabContent><Users {tag} /></TabContent>
    <TabContent>
      <DefaultTribes {tag} />
    </TabContent>
  </svelte:fragment>
</Tabs>
