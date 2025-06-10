<script lang="ts">
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
    TextInput,
  } from "carbon-components-svelte";
  import { getImageVersion, handleGetImageTags } from "./helpers/swarm";
  import { selectedNode, stack } from "./store";
  import {
    get_boltwall_max_request_limit,
    get_boltwall_request_per_seconds,
    update_boltwall_max_request_limit,
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
  let maxRequestLimit = "";
  let storedMaxRequestLimit = "";

  onMount(async () => {
    await handleGetRequestPerSeconds();
    await handleGetMaxRequestLimit();
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

  async function handleGetMaxRequestLimit() {
    try {
      const mrl = await get_boltwall_max_request_limit();
      if (mrl && mrl.success) {
        maxRequestLimit = mrl.data;
        storedMaxRequestLimit = mrl.data;
      }
    } catch (error) {
      console.log("Error getting boltwall max request limit: ", error);
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

  async function handleUpdateBoltwallMaxRequestLimit() {
    isLoading = true;
    const validatedMaxRequestLimit = validateMaxRequestLimit(maxRequestLimit);
    if (!validatedMaxRequestLimit) {
      notification_message = "Invalid Max Request Limit value";
      isLoading = false;
      show_notification = true;
      return;
    }
    maxRequestLimit = validatedMaxRequestLimit;
    try {
      const res = await update_boltwall_max_request_limit({
        max_request_limit: maxRequestLimit,
      });
      notification_message = res?.message;
      if (res.success) {
        success = true;
        storedMaxRequestLimit = maxRequestLimit;
      }
    } catch (error) {
      console.log(error);
    } finally {
      isLoading = false;
      show_notification = true;
    }
  }

  function validateMaxRequestLimit(size: string) {
    const transaformedSize = size.toLocaleLowerCase();
    if (/^(?!0)(\d+)(kb|mb)$/i.test(transaformedSize)) {
      return transaformedSize;
    } else {
      return "";
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
          <div class="updateMaxRequestLimit">
            <TextInput
              bind:value={maxRequestLimit}
              labelText={"Max Request Limit"}
              placeholder={"Enter size (e.g., 500KB or 2MB)"}
              type="text"
            />
            <Button
              on:click={handleUpdateBoltwallMaxRequestLimit}
              disabled={!maxRequestLimit ||
                storedMaxRequestLimit === maxRequestLimit}
              >Update Request Max Limit</Button
            >
          </div>
        </TabContent>
        <TabContent>
          <EnvContainer />
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
    margin-top: 1.5rem;
  }

  .updateReqPerSecs {
    margin-top: 1.2rem;
    display: flex;
    flex-direction: column;
    row-gap: 0.5rem;
  }

  .updateMaxRequestLimit {
    margin-top: 1.2rem;
    display: flex;
    flex-direction: column;
    row-gap: 0.5rem;
  }
</style>
