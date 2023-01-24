<script>
  import Users from "./Users.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import DefaultTribes from "./DefaultTribes.svelte";
</script>

<Tabs>
  <Tab label="Users" />
  <Tab label="Configuration" />
  <svelte:fragment slot="content">
    <TabContent><Users /></TabContent>
    <TabContent>
      <DefaultTribes />
    </TabContent>
  </svelte:fragment>
</Tabs>
