<script>
    import Tribes from "./Tribes.svelte";
    import People from "./People.svelte";
    import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  </script>
  
  <Tabs>
    <Tab label="Tribes" />
    <Tab label="People" />
    <svelte:fragment slot="content">
      <TabContent><Tribes /></TabContent>
      <TabContent><People /></TabContent>
    </svelte:fragment>
  </Tabs>
  