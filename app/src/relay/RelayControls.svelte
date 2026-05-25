<script>
  import Users from "./Users.svelte";
  import { Tabs, Tab, TabContent } from "carbon-components-svelte";
  import Admin from "./Admin.svelte";
  import { finishedOnboarding, isOnboarding } from "../store";
  import EnvContainer from "../components/envContainer/index.svelte";

  export let tag = "";

  $: selected = 0;
  $: $finishedOnboarding, selectCurrentTab();
  function selectCurrentTab() {
    if ($isOnboarding) {
      if ($finishedOnboarding.hasChannels && !$finishedOnboarding.hasAdmin) {
        selected = 1;
      }
      if (
        $finishedOnboarding.hasAdmin &&
        $finishedOnboarding.hasChannels &&
        !$finishedOnboarding.hasUsers
      ) {
        selected = 0;
      }
    }
  }
</script>

<Tabs bind:selected>
  <Tab label="Users" />
  <Tab label="Configuration" />
  <Tab label="Advance" />
  <svelte:fragment slot="content">
    <TabContent><Users {tag} /></TabContent>
    <TabContent>
      <Admin {tag} />
    </TabContent>
    <TabContent><EnvContainer /></TabContent>
  </svelte:fragment>
</Tabs>
