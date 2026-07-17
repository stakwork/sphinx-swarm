<script lang="ts">
  import { onMount } from "svelte";
  import { anthropicKeys, remotes } from "./store";
  import {
    Button,
    Checkbox,
    InlineNotification,
    Loading,
    Modal,
    Select,
    SelectItem,
    TextArea,
    TextInput,
    ToastNotification,
  } from "carbon-components-svelte";
  import KeyDisplayCard from "./components/KeyDisplayCard.svelte";
  import {
    add_anthropic_key,
    get_anthropic_keys,
    get_child_swarm_llm_keys,
    update_child_swarm_env,
  } from "../../../../../app/src/api/swarm";

  const LLM_PROVIDERS = [
    { name: "Anthropic", env: "ANTHROPIC_API_KEY" },
    { name: "OpenAI", env: "OPENAI_API_KEY" },
    { name: "Google", env: "GOOGLE_API_KEY" },
    { name: "OpenRouter", env: "OPENROUTER_API_KEY" },
  ];

  let error_message = "";
  let loading = false;
  let anthropicKey = "";
  let openAddAnthropicModal = false;
  let isSubmitting = false;
  let error_notification = false;
  let message;

  // distribute keys modal
  let open_distribute = false;
  let distribute_env = "ANTHROPIC_API_KEY";
  let distribute_keys_text = "";
  let selected_hosts: { [host: string]: boolean } = {};
  let distributing = false;
  let distribute_results: {
    host: string;
    success: boolean;
    message: string;
  }[] = [];

  $: distribute_keys = distribute_keys_text
    .split("\n")
    .map((k) => k.trim())
    .filter((k) => k.length > 0);
  $: selected_host_list = Object.keys(selected_hosts).filter(
    (h) => selected_hosts[h]
  );
  // one key + many swarms = rotation (same key everywhere)
  $: rotate_mode = distribute_keys.length === 1 && selected_host_list.length > 1;
  $: distribute_valid =
    selected_host_list.length > 0 &&
    (rotate_mode || distribute_keys.length >= selected_host_list.length);

  function withTimeout(promise, ms) {
    return Promise.race([
      promise,
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error("timeout")), ms)
      ),
    ]);
  }

  // fleet coverage
  let coverage_loading = false;
  let coverage_loaded = false;
  let coverage: {
    host: string;
    keys?: { [k: string]: string };
    error?: string;
  }[] = [];
  let duplicate_keys: { [k: string]: boolean } = {};

  onMount(async () => {
    await handleGetAnthropicKeys();
  });

  async function handleGetAnthropicKeys() {
    loading = true;
    try {
      let res = await get_anthropic_keys();
      if (res.success) {
        if (res.data) {
          anthropicKeys.set(res.data);
        }
      } else {
        error_message = res.message;
      }
    } catch (error) {
      console.log("error: ", error);
      error_message = "Error occured while trying to get anthropic keys";
    } finally {
      loading = false;
    }
  }

  async function handleAddAnthropicKey() {
    try {
      isSubmitting = true;
      if (!anthropicKey.trim()) {
        message = "Please provide a valid anthropic key";
        error_notification = true;
        return;
      }
      let res = await add_anthropic_key({ key: anthropicKey.trim() });
      if (res.success) {
        handleClosenAddAnthropicKeyModal();
        handleGetAnthropicKeys();
      } else {
        message = res.message;
      }
    } catch (error) {
      console.log("error: ", error);
      message = "Error occured while trying to add anthropic keys";
    } finally {
      isSubmitting = false;
    }
  }

  function handleClosenAddAnthropicKeyModal() {
    anthropicKey = "";
    openAddAnthropicModal = false;
  }

  function handleOpenDistribute() {
    distribute_results = [];
    open_distribute = true;
  }

  function handleCloseDistribute() {
    open_distribute = false;
    distribute_keys_text = "";
    selected_hosts = {};
    distribute_results = [];
    distributing = false;
  }

  function toggleSelectAll(e) {
    const checked = e.detail;
    const next = {};
    for (const r of $remotes) {
      next[r.host] = checked;
    }
    selected_hosts = next;
  }

  async function distributeToHost(host: string, key: string) {
    try {
      // env update restarts the swarm's containers, so allow a couple of
      // minutes — but never hang on a dead swarm
      const res = await withTimeout(
        update_child_swarm_env({ host, envs: { [distribute_env]: key } }),
        180000
      );
      distribute_results = [
        ...distribute_results,
        { host, success: res.success, message: res.message },
      ];
    } catch (error) {
      const timed_out = error && error.message === "timeout";
      distribute_results = [
        ...distribute_results,
        {
          host,
          success: false,
          message: timed_out
            ? "timed out — swarm may be down (or still applying the update)"
            : "request failed",
        },
      ];
    }
  }

  async function handleDistribute() {
    if (distributing || !distribute_valid) return;
    distributing = true;
    distribute_results = [];
    const hosts = selected_host_list;
    const batch_size = 3;
    for (let i = 0; i < hosts.length; i += batch_size) {
      const batch = hosts.slice(i, i + batch_size);
      await Promise.all(
        batch.map((host, j) =>
          distributeToHost(host, rotate_mode ? distribute_keys[0] : distribute_keys[i + j])
        )
      );
    }
    distributing = false;
    distribute_keys_text = "";
  }

  async function handleCheckCoverage() {
    coverage_loading = true;
    coverage = [];
    const hosts = $remotes.map((r) => r.host);
    const batch_size = 5;
    let results = [];
    for (let i = 0; i < hosts.length; i += batch_size) {
      const batch = hosts.slice(i, i + batch_size);
      const batch_results = await Promise.all(
        batch.map(async (host) => {
          try {
            const res = await withTimeout(
              get_child_swarm_llm_keys({ host }),
              45000
            );
            if (res.success && res.data) {
              return { host, keys: res.data.keys };
            }
            return { host, error: res.message || "no data" };
          } catch (error) {
            const timed_out = error && error.message === "timeout";
            return {
              host,
              error: timed_out ? "timed out — swarm may be down" : "request failed",
            };
          }
        })
      );
      results = [...results, ...batch_results];
      coverage = results;
    }
    // flag keys shared by more than one swarm
    const counts: { [k: string]: number } = {};
    for (const row of results) {
      if (!row.keys) continue;
      for (const env of Object.keys(row.keys)) {
        const id = `${env}|${row.keys[env]}`;
        counts[id] = (counts[id] || 0) + 1;
      }
    }
    duplicate_keys = {};
    for (const id of Object.keys(counts)) {
      if (counts[id] > 1) duplicate_keys[id] = true;
    }
    coverage = results;
    coverage_loaded = true;
    coverage_loading = false;
  }

  function isDuplicate(env: string, masked: string) {
    return duplicate_keys[`${env}|${masked}`] === true;
  }
</script>

<main>
  <div class="keys_card_container">
    {#if loading}
      <Loading />
    {/if}
    {#if error_message}
      <div class="success_toast_container">
        <ToastNotification
          lowContrast
          kind={"error"}
          title={"Error"}
          subtitle={error_message}
          fullWidth={true}
        />
      </div>
    {/if}

    <div class="section_header">
      <div>
        <h4>Anthropic key pool</h4>
        <p class="section_hint">
          Unassigned keys, consumed when a new swarm is created
        </p>
      </div>
      <Button on:click={() => (openAddAnthropicModal = true)}
        >Add Anthropic Key</Button
      >
    </div>
    {#if $anthropicKeys.length > 0}
      {#each $anthropicKeys as key}
        <KeyDisplayCard value={key} />
      {/each}
    {:else}
      <p class="empty_hint">No Anthropic keys in the pool</p>
    {/if}

    <div class="section_header">
      <div>
        <h4>Distribute keys</h4>
        <p class="section_hint">
          Push individual API keys to swarms. Keys are written straight to each
          swarm's .env and are not stored on the superadmin.
        </p>
      </div>
      <Button on:click={handleOpenDistribute}>Distribute Keys</Button>
    </div>

    <div class="section_header">
      <div>
        <h4>Fleet coverage</h4>
        <p class="section_hint">
          Live per-swarm LLM key status, fetched from each swarm on demand
        </p>
      </div>
      <Button
        kind="secondary"
        disabled={coverage_loading}
        on:click={handleCheckCoverage}
        >{coverage_loading ? "Checking..." : "Check Coverage"}</Button
      >
    </div>
    {#if coverage.length > 0}
      <table class="coverage_table">
        <thead>
          <tr>
            <th>Swarm</th>
            {#each LLM_PROVIDERS as provider}
              <th>{provider.name}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each coverage as row}
            <tr>
              <td class="mono">{row.host}</td>
              {#if row.error}
                <td colspan="4" class="error_cell">{row.error}</td>
              {:else}
                {#each LLM_PROVIDERS as provider}
                  {#if row.keys && row.keys[provider.env]}
                    <td
                      class="mono {isDuplicate(
                        provider.env,
                        row.keys[provider.env]
                      )
                        ? 'shared_key'
                        : 'has_key'}"
                      >{row.keys[provider.env]}{isDuplicate(
                        provider.env,
                        row.keys[provider.env]
                      )
                        ? " (shared)"
                        : ""}</td
                    >
                  {:else}
                    <td class="empty_cell">—</td>
                  {/if}
                {/each}
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    {:else if coverage_loaded}
      <p class="empty_hint">No swarms found</p>
    {/if}
  </div>

  <Modal
    bind:open={openAddAnthropicModal}
    modalHeading="Add Anthropic Key"
    primaryButtonDisabled={!anthropicKey || isSubmitting}
    primaryButtonText={isSubmitting ? "Loading..." : "Add"}
    secondaryButtonText="Cancel"
    on:click:button--secondary={handleClosenAddAnthropicKeyModal}
    on:open
    on:close={handleClosenAddAnthropicKeyModal}
    on:submit={handleAddAnthropicKey}
  >
    {#if error_notification}
      <InlineNotification
        kind="error"
        title="Error:"
        subtitle={message}
        timeout={8000}
        on:close={(e) => {
          e.preventDefault();
          error_notification = false;
        }}
      />
    {/if}
    <div class="input_container">
      <TextInput
        labelText="Anthropic Key"
        placeholder="Enter Anthropic Key..."
        bind:value={anthropicKey}
      />
    </div>
  </Modal>

  <Modal
    bind:open={open_distribute}
    modalHeading="Distribute LLM Keys"
    primaryButtonDisabled={distributing || !distribute_valid}
    primaryButtonText={distributing
      ? "Distributing..."
      : rotate_mode
        ? "Rotate All"
        : "Distribute"}
    secondaryButtonText="Close"
    preventCloseOnClickOutside
    on:click:button--secondary={handleCloseDistribute}
    on:close={handleCloseDistribute}
    on:submit={handleDistribute}
  >
    <div class="input_container">
      <Select labelText="Provider" bind:selected={distribute_env}>
        {#each LLM_PROVIDERS as provider}
          <SelectItem value={provider.env} text={provider.name} />
        {/each}
      </Select>
    </div>
    <div class="input_container">
      <TextArea
        labelText="API keys (one per line)"
        placeholder="sk-..."
        rows={4}
        bind:value={distribute_keys_text}
      />
    </div>
    <div class="input_container">
      <p class="swarm_select_label">
        {#if rotate_mode}
          Rotation: the single key will be applied to all {selected_host_list.length}
          selected swarms
        {:else}
          The first key goes to the first selected swarm, and so on ({distribute_keys.length}
          keys / {selected_host_list.length} selected). Paste a single key to
          rotate the same key onto every selected swarm.
        {/if}
      </p>
      <Checkbox
        labelText="Select all"
        checked={$remotes.length > 0 &&
          selected_host_list.length === $remotes.length}
        on:check={toggleSelectAll}
      />
      <div class="swarm_checkbox_list">
        {#each $remotes as remote}
          <Checkbox
            labelText={remote.host}
            bind:checked={selected_hosts[remote.host]}
          />
        {/each}
      </div>
    </div>
    <InlineNotification
      hideCloseButton
      lowContrast
      kind="warning"
      title="Heads up:"
      subtitle="each swarm's containers restart when its env is updated."
    />
    {#if distribute_results.length > 0}
      <div class="distribute_results">
        {#each distribute_results as result}
          <p class={result.success ? "result_ok" : "result_fail"}>
            {result.success ? "✓" : "✗"}
            {result.host} — {result.message}
          </p>
        {/each}
      </div>
    {/if}
  </Modal>
</main>

<style>
  .keys_card_container {
    display: flex;
    flex-direction: column;
    padding: 1.5rem;
  }

  .section_header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin: 1.5rem 0 0.75rem 0;
    gap: 1rem;
  }

  .section_header h4 {
    margin: 0;
  }

  .section_hint {
    font-size: 0.8rem;
    color: #8a9ba8;
    margin: 0.2rem 0 0 0;
  }

  .success_toast_container {
    margin-bottom: 2rem;
  }

  .empty_hint {
    color: #8a9ba8;
    font-size: 0.9rem;
  }

  .input_container {
    margin-bottom: 1rem;
  }

  .swarm_select_label {
    font-size: 0.75rem;
    color: #8a9ba8;
    margin-bottom: 0.3rem;
  }

  .swarm_checkbox_list {
    max-height: 12rem;
    overflow-y: auto;
    border: 1px solid #394b59;
    border-radius: 0.25rem;
    padding: 0.4rem 0.6rem;
    margin-top: 0.3rem;
  }

  .distribute_results {
    margin-top: 1rem;
    max-height: 10rem;
    overflow-y: auto;
  }

  .distribute_results p {
    margin: 0.2rem 0;
    font-size: 0.85rem;
  }

  .result_ok {
    color: #42be65;
  }

  .result_fail {
    color: #fa4d56;
  }

  .coverage_table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .coverage_table th {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #394b59;
    color: #8a9ba8;
    font-weight: 500;
  }

  .coverage_table td {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #29353f;
  }

  .mono {
    font-family: monospace;
  }

  .has_key {
    color: #42be65;
  }

  .shared_key {
    color: #f1c21b;
  }

  .empty_cell {
    color: #5f6f7d;
  }

  .error_cell {
    color: #fa4d56;
  }
</style>
