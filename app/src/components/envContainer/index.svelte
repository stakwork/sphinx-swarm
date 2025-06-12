<script lang="ts">
  import { onMount } from "svelte";
  import EnvRow from "./envRow.svelte";
  import { selectedNode } from "../../store";
  import { get_env_variables, update_env_variables } from "../../api/swarm";
  import { formatEnv } from "../../helpers/env";
  import { Button, Loading } from "carbon-components-svelte";

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

      if (response.success) {
        isChanged = false;
        await getEnvValues();
      }
    } catch (error) {
      console.log("Error updating environment variables:", error);
    }

    isLoading = false;
  }
</script>

<div class="container">
  {#if isLoading}
    <Loading />
  {/if}

  <div class="button_container">
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
  }

  .env_container {
    display: flex;
    flex-direction: column;
    height: 30rem;
    overflow-y: auto;
  }
</style>
