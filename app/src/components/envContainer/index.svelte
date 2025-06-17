<script lang="ts">
  import { onMount } from "svelte";
  import EnvRow from "./envRow.svelte";
  import { selectedNode } from "../../store";
  import {
    get_env_variables,
    update_env_variables,
    update_swarm,
  } from "../../api/swarm";
  import { formatEnv } from "../../helpers/env";
  import {
    Button,
    InlineNotification,
    Loading,
    Modal,
    TextInput,
  } from "carbon-components-svelte";

  interface EnvKeyValue {
    key: string;
    value: string;
  }
  interface TrackedEnvKeyValue extends EnvKeyValue {
    isChanged: boolean;
  }
  let EnvArray: EnvKeyValue[] = [];
  let isLoading = false;
  const envObjects: { [k: string]: EnvKeyValue } = {};
  const trackEnvObjects: { [k: string]: TrackedEnvKeyValue } = {};
  let isChanged = false;
  let notificationMessage = "";
  let isSuccessNotfication = false;
  let show_notification = false;
  let openAddEnv = false;
  let newKey = "";
  let newValue = "";
  let isSubmitting = false;
  let addEnvErrorNotification = false;

  async function getEnvValues() {
    const env_var = await get_env_variables($selectedNode.name);
    if (env_var.success) {
      EnvArray = formatEnv(env_var.data);
    }

    EnvArray.forEach((env) => {
      envObjects[env.key] = { ...env };
      trackEnvObjects[env.key] = { ...env, isChanged: false };
    });
  }

  function handleOpenAddEnv() {
    openAddEnv = true;
  }

  onMount(async () => {
    isLoading = true;
    await getEnvValues();
    isLoading = false;
  });
  // have the initial env value as object
  // have another object that I will use to track changes

  function handleEnvChange(key: string, value: string) {
    const current = trackEnvObjects[key];
    if (!current) {
      console.error(`Environment variable with key ${key} not found.`);
      return;
    }
    if (envObjects[key].value !== value) {
      trackEnvObjects[key] = { ...current, value, isChanged: true };
    } else {
      trackEnvObjects[key] = { ...current, value, isChanged: false };
    }
    isChanged = Object.values(trackEnvObjects).some((env) => env.isChanged);
  }

  function handleOnCloseAddEnv() {
    openAddEnv = false;
    newKey = "";
    newValue = "";
    addEnvErrorNotification = false;
  }

  async function handleSubmitAddEnv() {
    isSubmitting = true;

    try {
      const response = await update_env_variables({
        id: $selectedNode.name,
        values: { [newKey]: newValue },
      });
      notificationMessage = response.message;
      if (response.success) {
        isSuccessNotfication = true;
        show_notification = true;
        notificationMessage = `${notificationMessage}. Server will restart to apply changes.`;
        isSubmitting = false;
        handleOnCloseAddEnv();
        await update_swarm();
      } else {
        addEnvErrorNotification = true;
        isSubmitting = false;
      }
    } catch (error) {
      console.error("Error in handleSubmitAddEnv:", error);
      isSubmitting = false;
    }
  }

  async function submitEnvChange() {
    isLoading = true;
    const updateEnvArray = Object.values(trackEnvObjects);
    const updatedEnvObj = {};
    for (let i = 0; i < updateEnvArray.length; i++) {
      const env = updateEnvArray[i];
      if (env.isChanged) {
        updatedEnvObj[env.key] = env.value;
      }
    }
    // make API call to the backend.
    try {
      const response = await update_env_variables({
        id: $selectedNode.name,
        values: updatedEnvObj,
      });
      notificationMessage = response.message;
      if (response.success) {
        isSuccessNotfication = true;
        isChanged = false;
        notificationMessage = `${notificationMessage}. Server will restart to apply changes.`;
        show_notification = true;
        isLoading = false;
        await update_swarm();
      } else {
        show_notification = true;
        isLoading = false;
      }
    } catch (error) {
      console.error("Error updating environment variables:", error);
      notificationMessage = "Failed to update environment variables.";
      isSuccessNotfication = false;
      show_notification = true;
      isLoading = false;
    }
  }
</script>

<div class="container">
  {#if isLoading}
    <Loading />
  {/if}
  {#if show_notification}
    <InlineNotification
      lowContrast
      kind={isSuccessNotfication ? "success" : "error"}
      title={isSuccessNotfication ? "Success:" : "Error:"}
      subtitle={notificationMessage}
      timeout={3000}
      on:close={(e) => {
        e.preventDefault();
        show_notification = false;
      }}
    />
  {/if}
  <div class="button_container">
    <Button on:click={handleOpenAddEnv}>Add Env</Button>
    <Button on:click={submitEnvChange} disabled={!isChanged}>Update Env</Button>
  </div>
  <div class="env_container">
    {#each EnvArray as { key, value }}
      <EnvRow
        {key}
        {value}
        on:update={(e) => handleEnvChange(e.detail.key, e.detail.value)}
      />
    {/each}
  </div>
  <Modal
    bind:open={openAddEnv}
    modalHeading="Add Environment Variable"
    primaryButtonDisabled={!newKey || !newValue || isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Update"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={() => (openAddEnv = false)}
    on:close={handleOnCloseAddEnv}
    on:submit={handleSubmitAddEnv}
  >
    {#if addEnvErrorNotification}
      <InlineNotification
        kind={isSuccessNotfication ? "success" : "error"}
        title={isSuccessNotfication ? "Success:" : "Error:"}
        subtitle={notificationMessage}
        timeout={8000}
        on:close={(e) => {
          e.preventDefault();
          addEnvErrorNotification = false;
        }}
      />
    {/if}
    <div class="input_container">
      <TextInput
        labelText="Environment Variable Key"
        placeholder="Enter Environment Variable Key..."
        bind:value={newKey}
      />
      <TextInput
        labelText="Environment Variable Value"
        placeholder="Enter Environment Variable Value..."
        bind:value={newValue}
      />
    </div>
  </Modal>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding-top: 1rem;
    padding-bottom: 1rem;
  }

  .button_container {
    display: flex;
    justify-content: flex-end;
    padding-right: 1rem;
    gap: 2rem;
  }

  .env_container {
    display: flex;
    flex-direction: column;
    height: 30rem;
    overflow-y: auto;
  }

  .input_container {
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }
</style>
