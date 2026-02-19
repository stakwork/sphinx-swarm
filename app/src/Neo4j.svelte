<script lang="ts">
  import { Loading, PasswordInput, NumberInput, Button } from "carbon-components-svelte";
  import {
    get_neo4j_password,
    update_neo4j_config,
    get_config,
    get_env_variables,
  } from "./api/swarm";
  import { stack } from "./store";
  import type { Node, Stack } from "./nodes";
  import { onMount } from "svelte";

  const NEO4J_CONTAINER_NAME = "neo4j";

  const DEFAULTS = {
    heap_initial_gb: 6,
    heap_max_gb: 6,
    pagecache_gb: 8,
    tx_total_gb: 4,
    tx_max_gb: 1,
    checkpoint_iops: 500,
  };

  /* -----------------------------
     Utility Helpers
  ------------------------------*/

  function safeNumber(v: unknown): number | null {
    const n = Number(v);
    return Number.isFinite(n) && n >= 1 ? n : null;
  }

  /**
   * Parse Neo4j env values:
   * 6g, 6GB, 6144m, 6144MB, 6
   */
  function parseEnvValue(raw: string): number | null {
    if (!raw) return null;

    const s = String(raw).trim().toLowerCase();

    if (s.endsWith("gb")) {
      return safeNumber(s.replace("gb", ""));
    }

    if (s.endsWith("g")) {
      return safeNumber(s.replace("g", ""));
    }

    if (s.endsWith("mb")) {
      const n = safeNumber(s.replace("mb", ""));
      return n ? n / 1024 : null;
    }

    if (s.endsWith("m")) {
      const n = safeNumber(s.replace("m", ""));
      return n ? n / 1024 : null;
    }

    return safeNumber(s);
  }

  function getFromEnv(env: Record<string, string>, keys: string[]): number | null {
    for (const k of keys) {
      const raw = env[k];
      if (raw != null && raw !== "") {
        const parsed = parseEnvValue(raw);
        if (parsed !== null) return parsed;
      }
    }
    return null;
  }

  function getNeo4jSettingsFromEnv(env: Record<string, string>) {
    const heap_initial = getFromEnv(env, [
      "NEO4J_server_memory_heap_initial__size",
      "NEO4J_dbms_memory_heap_initial__size",
    ]);

    const heap_max = getFromEnv(env, [
      "NEO4J_server_memory_heap_max__size",
      "NEO4J_dbms_memory_heap_max__size",
    ]);

    const pagecache = getFromEnv(env, [
      "NEO4J_server_memory_pagecache_size",
      "NEO4J_dbms_memory_pagecache_size",
    ]);

    const tx_total = getFromEnv(env, ["NEO4J_db_memory_transaction_total_max"]);
    const tx_max = getFromEnv(env, ["NEO4J_db_memory_transaction_max"]);
    const checkpoint = getFromEnv(env, ["NEO4J_db_checkpoint_iops_limit"]);

    if (
      heap_initial === null &&
      heap_max === null &&
      pagecache === null &&
      tx_total === null &&
      tx_max === null &&
      checkpoint === null
    ) {
      return null;
    }

    return {
      heap_initial_gb: heap_initial ?? DEFAULTS.heap_initial_gb,
      heap_max_gb: heap_max ?? DEFAULTS.heap_max_gb,
      pagecache_gb: pagecache ?? DEFAULTS.pagecache_gb,
      tx_total_gb: tx_total ?? DEFAULTS.tx_total_gb,
      tx_max_gb: tx_max ?? DEFAULTS.tx_max_gb,
      checkpoint_iops: checkpoint ?? DEFAULTS.checkpoint_iops,
    };
  }

  function getNeo4jSettingsFromNode(node: Node | undefined) {
    if (!node) return DEFAULTS;

    return {
      heap_initial_gb: safeNumber(node.heap_initial_gb) ?? DEFAULTS.heap_initial_gb,
      heap_max_gb: safeNumber(node.heap_max_gb) ?? DEFAULTS.heap_max_gb,
      pagecache_gb: safeNumber(node.pagecache_gb) ?? DEFAULTS.pagecache_gb,
      tx_total_gb: safeNumber(node.tx_total_gb) ?? DEFAULTS.tx_total_gb,
      tx_max_gb: safeNumber(node.tx_max_gb) ?? DEFAULTS.tx_max_gb,
      checkpoint_iops: safeNumber(node.checkpoint_iops) ?? DEFAULTS.checkpoint_iops,
    };
  }

  function applySettings(settings: typeof DEFAULTS) {
    heap_initial_gb = settings.heap_initial_gb;
    heap_max_gb = settings.heap_max_gb;
    pagecache_gb = settings.pagecache_gb;
    tx_total_gb = settings.tx_total_gb;
    tx_max_gb = settings.tx_max_gb;
    checkpoint_iops = settings.checkpoint_iops;
  }

  function applySettingsFromStack(s: Stack) {
    const neo = s.nodes.find((n) => n.type === "Neo4j");
    applySettings(getNeo4jSettingsFromNode(neo));
  }

  /* -----------------------------
     State
  ------------------------------*/

  let isLoading = true;
  let neo4jPassword = "Loading Neo4j Password";

  let heap_initial_gb = DEFAULTS.heap_initial_gb;
  let heap_max_gb = DEFAULTS.heap_max_gb;
  let pagecache_gb = DEFAULTS.pagecache_gb;
  let tx_total_gb = DEFAULTS.tx_total_gb;
  let tx_max_gb = DEFAULTS.tx_max_gb;
  let checkpoint_iops = DEFAULTS.checkpoint_iops;

  let isSaving = false;
  let saveMessage = "";

  /* -----------------------------
     API Calls
  ------------------------------*/

  async function handleGetNeo4jPassword() {
    try {
      const res = await get_neo4j_password();
      if (res.success === true) {
        neo4jPassword = res.data;
      }
    } catch (error) {
      console.error("Error getting neo4j password", error);
    }
  }

  async function handleUpdateNeo4jConfig() {
    isSaving = true;
    saveMessage = "";

    try {
      const res = await update_neo4j_config({
        heap_initial_gb,
        heap_max_gb,
        pagecache_gb,
        tx_total_gb,
        tx_max_gb,
        checkpoint_iops,
      });

      if (res?.success) {
        saveMessage =
          res.message ||
          "Neo4j config updated. Restart neo4j container to apply changes.";

        const updated: Stack = await get_config();
        if (updated?.nodes) {
          stack.set(updated);
          applySettingsFromStack(updated);
        }
      } else {
        saveMessage = res?.message || "Error updating neo4j config";
      }
    } catch (e) {
      console.error("Error updating neo4j config", e);
      saveMessage = "Error updating neo4j config";
    } finally {
      isSaving = false;
    }
  }

  async function loadSettings() {
    try {
      const res = await get_env_variables(NEO4J_CONTAINER_NAME);

      if (res?.success && res?.data && typeof res.data === "object") {
        const fromEnv = getNeo4jSettingsFromEnv(res.data);
        if (fromEnv) {
          applySettings(fromEnv);
          return;
        }
      }

      applySettingsFromStack($stack);
    } catch (e) {
      console.warn("[Neo4j] Failed to load ENV, using stack config", e);
      applySettingsFromStack($stack);
    }
  }

  onMount(async () => {
    await loadSettings();
    await handleGetNeo4jPassword();
    isLoading = false;
  });
</script>

<div class="nav-wrapper">
  {#if isLoading}
    <Loading />
  {/if}

  <div class="neo4j_container">
    <PasswordInput labelText="Password" value={neo4jPassword} readonly />

    <div class="settings">
      <h3>Neo4j Memory & IO Settings</h3>

      <div class="grid">
        <NumberInput id="heap_initial_gb" label="Heap initial size (GB)" min={1} bind:value={heap_initial_gb} />
        <NumberInput id="heap_max_gb" label="Heap max size (GB)" min={1} bind:value={heap_max_gb} />
        <NumberInput id="pagecache_gb" label="Page cache size (GB)" min={1} bind:value={pagecache_gb} />
        <NumberInput id="tx_total_gb" label="Tx total memory max (GB)" min={1} bind:value={tx_total_gb} />
        <NumberInput id="tx_max_gb" label="Tx memory max (GB)" min={1} bind:value={tx_max_gb} />
        <NumberInput id="checkpoint_iops" label="Checkpoint IOPS limit" min={1} bind:value={checkpoint_iops} />
      </div>

      <Button kind="primary" disabled={isSaving} on:click={handleUpdateNeo4jConfig}>
        {isSaving ? "Saving..." : "Save Neo4j Config"}
      </Button>

      {#if saveMessage}
        <p class="save-message">{saveMessage}</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .nav-wrapper {
    font-size: 1rem;
    padding: 0px 25px;
  }

  .neo4j_container {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    max-width: 600px;
  }

  .settings .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .save-message {
    margin-top: 0.5rem;
  }
</style>
