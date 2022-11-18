<script>
    import Users from "./Users.svelte";
    import { Tabs, Tab, TabContent } from "carbon-components-svelte";
    import { selectedNode } from "./store";
    import Controls from "./Controls.svelte";
    import { controls } from "./controls";
  </script>
  
  <Tabs>
    <Tab label="Tribes" />
    <Tab label="Configuration" />
    <svelte:fragment slot="content">
      <TabContent><Users /></TabContent>
      <TabContent>
        <Controls ctrls={$selectedNode && controls["Tribes"]} />
      </TabContent>
    </svelte:fragment>
  </Tabs>