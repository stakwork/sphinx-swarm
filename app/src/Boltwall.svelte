<script>
  import { onMount } from "svelte";
  import {
    Select,
    SelectItem,
    Button,
    Loading,
    InlineNotification,
    NumberInput,
    Tabs,
    Tab,
    TabContent,
  } from "carbon-components-svelte";
  import { getImageVersion, handleGetImageTags } from "./helpers/swarm";
  import { selectedNode, stack } from "./store";
  import {
    get_boltwall_request_per_seconds,
    get_env_variables,
    update_boltwall_request_per_seconds,
    update_node,
  } from "./api/swarm";
  import EnvContainer from "./components/envContainer/index.svelte";
  import { formatEnv } from "./helpers/env";
  export let host = "";
  export let updateBody = () => {};
  let link = host ? `https://${host}` : "http://localhost:8444";
  let tags = [];
  let selected_tag = "";
  let isLoading = true;
  let show_notification = false;
  let notification_message = "";
  let success = false;
  let requestPerSeconds = 0;
  let storedRequestPerSeconds = 0;
  let envs = [];

  onMount(async () => {
    const env_var = await get_env_variables($selectedNode.name);
    if (env_var.success) {
      envs = formatEnv(env_var.data);
    }
    await handleGetRequestPerSeconds();
    tags = await handleGetImageTags($selectedNode.name);
    isLoading = false;
  });

  async function handleGetRequestPerSeconds() {
    try {
      const rps = await get_boltwall_request_per_seconds();
      if (rps && rps.success) {
        requestPerSeconds = rps.data;
        storedRequestPerSeconds = rps.data;
      }
    } catch (error) {
      console.log("Error getting boltwall request per seconds: ", error);
    }
  }

  async function handleUpdateNodeVersion() {
    isLoading = true;
    try {
      const res = await update_node($selectedNode.name, selected_tag);
      if (res === "{}") {
        await getImageVersion(stack, selectedNode);
        updateBody();
        success = true;
        notification_message = `${$selectedNode.name} version updated successfully`;
      }
    } catch (error) {
      console.log(error);
    } finally {
      isLoading = false;
      show_notification = true;
    }
  }

  async function handleUpdateBoltwallRequestPerSeconds() {
    isLoading = true;
    try {
      const res = await update_boltwall_request_per_seconds({
        request_per_seconds: Number(requestPerSeconds),
      });
      notification_message = res?.message;
      if (res.success) {
        success = true;
        storedRequestPerSeconds = Number(requestPerSeconds);
      }
    } catch (error) {
      console.log(error);
    } finally {
      isLoading = false;
      show_notification = true;
    }
  }
</script>

<div class="nav-wrapper">
  {#if show_notification}
    <InlineNotification
      lowContrast
      kind={success ? "success" : "error"}
      title={success ? "Success:" : "Error:"}
      subtitle={notification_message}
      timeout={9000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  <div class="title">Boltwall URL:</div>
  <div class="spacer" />
  <div>{link}</div>
  {#if isLoading}
    <Loading />
  {/if}
  <div class="tabContainer">
    <Tabs>
      <Tab label="General" />
      <Tab label="Advance" />
      <svelte:fragment slot="content">
        <TabContent>
          <div class="update_container">
            <Select
              labelText={`Update ${$selectedNode.name} version`}
              selected="g10"
              on:change={(e) => (selected_tag = e.target.value)}
            >
              {#each tags as tag}
                <SelectItem value={`${tag}`} text={`${tag}`} />
              {/each}
            </Select>
            <Button
              on:click={handleUpdateNodeVersion}
              disabled={!selected_tag || $selectedNode.version === selected_tag}
              >Update Version</Button
            >
          </div>
          <div class="updateReqPerSecs">
            <NumberInput
              label="Update Request Per seconds"
              bind:value={requestPerSeconds}
            />
            <Button
              on:click={handleUpdateBoltwallRequestPerSeconds}
              disabled={!requestPerSeconds ||
                storedRequestPerSeconds === requestPerSeconds}
              >Update Request Per Seconds</Button
            >
          </div>
        </TabContent>
        <TabContent>
          <EnvContainer EnvArray={envs} />
        </TabContent>
      </svelte:fragment>
    </Tabs>
  </div>
</div>

<style>
  .nav-wrapper {
    font-size: 1rem;
    padding: 0px 25px;
  }
  .title {
    margin-top: 1rem;
  }
  .spacer {
    height: 1rem;
  }

  .tabContainer {
    margin-top: 2rem;
  }
  .update_container {
    display: flex;
    gap: 1rem;
    flex-direction: column;
    margin-top: 2rem;
  }

  .updateReqPerSecs {
    margin-top: 2rem;
  }
</style>
