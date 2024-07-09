<script lang="ts">
  import {
    get_graph_accessibility,
    update_graph_accessibility,
    get_second_brain_about_details,
    get_feature_flag,
    update_second_brain_about,
    update_feature_flags,
  } from "../../api/swarm";
  import { onMount } from "svelte";

  $: isChange = false;
  $: isLoading = false;
  $: state = {
    graph_name: {
      value: "",
      method: async (title: string) => updateGraphDetails(title),
    },
    trendingTopics: {
      value: true,
    },
    public: {
      value: true,
      method: async (value: boolean) => toggleGraphStatus(value),
    },
    addItem: {
      value: true,
    },
    addContent: {
      value: true,
    },
    settings: {
      value: true,
    },
    chatInterface: {
      value: true,
    },
  };
  $: about = {};
  $: isSuccess = false;

  $: changedState = {
    graph_name: { value: "", isChange: false },
    trendingTopics: { value: true, isChange: false },
    public: { value: true, isChange: false },
    addItem: { value: true, isChange: false },
    addContent: { value: true, isChange: false },
    settings: { value: true, isChange: false },
    chatInterface: { value: true, isChange: false },
  };

  const featureFlags = [
    {
      key: "trendingTopics",
      label: "Trending Topics",
      description: "Toggle to display Trending topics on the graph.",
    },
    {
      key: "addItem",
      label: "Add Item",
      description: "Toggle Add Item on the Graph",
    },
    {
      key: "addContent",
      label: "Add Content",
      description: "Toggle Add Content on the Graph",
    },
    {
      key: "settings",
      label: "Settings",
      description: "Toggle Settings on the Graph",
    },
    {
      key: "chatInterface",
      label: "AI Summary",
      description: "Toggle AI Summary Feature flag",
    },
  ];

  function handleCheckBoxChange(e, value) {
    const checked = e.target.checked;
    if (checked !== state[value].value) {
      changedState = {
        ...changedState,
        [value]: { value: checked, isChange: true },
      };
    } else if (checked === state[value].value) {
      changedState = {
        ...changedState,
        [value]: { value: checked, isChange: false },
      };
    }

    //assume nothing changed and check
    isChange = false;

    //change the global value of isChange
    checkChangeState();
  }

  function checkChangeState() {
    for (let key in { ...changedState }) {
      if (changedState[key].isChange) {
        isChange = true;
      }
    }
  }

  function handleInputChange(e) {
    const inputValue = e.target.value;
    if (inputValue !== state.graph_name.value) {
      changedState = {
        ...changedState,
        graph_name: { value: inputValue, isChange: true },
      };
    } else if (inputValue === state.graph_name.value) {
      changedState = {
        ...changedState,
        graph_name: { value: inputValue, isChange: false },
      };
    }

    //assume nothing changed and check
    isChange = false;

    //change the global value of isChange
    checkChangeState();
  }

  async function toggleGraphStatus(value: boolean) {
    const result = await update_graph_accessibility(value);
    const parsedResult = JSON.parse(result);

    return { success: parsedResult.success, message: parsedResult.message };
  }

  //update graph name
  async function updateGraphDetails(title: string) {
    return await update_second_brain_about({ ...about, title });
  }

  //update trending topics
  async function updateTrendingTopic(value) {
    return await update_feature_flags({ trendingTopics: value });
  }

  async function discardButtonHandler() {
    for (let key in { ...changedState }) {
      changedState = {
        ...changedState,
        [key]: { value: state[key].value, isChange: false },
      };
    }

    isChange = false;
  }

  function findFeatureFlag(key: string) {
    for (let i = 0; i < featureFlags.length; i++) {
      if (featureFlags[i].key === key) {
        return true;
      }
    }
    return false;
  }

  async function handleSaveChanges() {
    isLoading = true;
    try {
      const changedFeatureFlag = {};
      for (let key in { ...changedState }) {
        if (changedState[key].isChange) {
          const isFeatureFlag = findFeatureFlag(key);
          if (isFeatureFlag) {
            changedFeatureFlag[key] = {
              user: changedState[key].value,
              admin: changedState[key].value,
            };
          } else {
            await state[key].method(changedState[key].value);
            //update state
            const newObj = { ...state[key], value: changedState[key].value };
            state = { ...state, [key]: { ...newObj } };
            changedState = {
              ...changedState,
              [key]: { value: changedState[key].value, isChange: false },
            };
          }
        }
      }

      if (Object.keys(changedFeatureFlag).length !== 0) {
        const result = await update_feature_flags({ ...changedFeatureFlag });
        const parsedResult = JSON.parse(result);
        if (parsedResult.success) {
          for (let key in changedFeatureFlag) {
            // Update The state
            state = {
              ...state,
              [key]: { value: changedFeatureFlag[key].user },
            };
            //  Updated Changes state
            changedState = {
              ...changedState,
              [key]: { value: changedState[key].value, isChange: false },
            };
          }
        }
      }

      //handle success state
      isSuccess = true;
      isLoading = false;
      isChange = false;

      setTimeout(() => {
        isSuccess = false;
      }, 5000);
    } catch (error) {
      isLoading = false;
    }
  }

  onMount(async () => {
    // get about details
    const aboutResult = await get_second_brain_about_details();
    const parsedAbout = await JSON.parse(aboutResult);
    about = { ...parsedAbout };

    // get trendingTopics feature flag state
    const featureFlagResult = await get_feature_flag();
    const parsedFeatureFlag = JSON.parse(featureFlagResult);

    // get public graph status
    const result = await get_graph_accessibility();
    const parsedResult = JSON.parse(result);

    //update state
    state = {
      public: {
        value: parsedResult.data.isPublic,
        method: async (value: boolean) => toggleGraphStatus(value),
      },
      graph_name: {
        value: parsedAbout.title,
        method: async (title: string) => updateGraphDetails(title),
      },
      trendingTopics: {
        value: parsedFeatureFlag.data.trendingTopics.user,
      },
      addItem: {
        value: parsedFeatureFlag.data.addItem.user,
      },
      addContent: {
        value: parsedFeatureFlag.data.addContent.user,
      },
      settings: {
        value: parsedFeatureFlag.data.settings.user,
      },
      chatInterface: {
        value: parsedFeatureFlag.data.chatInterface.user,
      },
    };

    //update changedState
    changedState = {
      public: { value: parsedResult.data.isPublic, isChange: false },
      graph_name: { value: parsedAbout.title, isChange: false },
      trendingTopics: {
        value: parsedFeatureFlag.data.trendingTopics.user,
        isChange: false,
      },
      addContent: {
        value: parsedFeatureFlag.data.addContent.user,
        isChange: false,
      },
      addItem: { value: parsedFeatureFlag.data.addItem.user, isChange: false },
      settings: {
        value: parsedFeatureFlag.data.settings.user,
        isChange: false,
      },
      chatInterface: {
        value: parsedFeatureFlag.data.chatInterface.user,
        isChange: false,
      },
    };
  });
</script>

<div class="container">
  <div class="header">
    <h2 class="title">General</h2>
    <div class="button-container">
      {#if isLoading === false && isChange === true && isSuccess === false}
        <button class="discard-button" on:click={discardButtonHandler}
          >Discard</button
        >
      {/if}
      {#if isSuccess === false}
        <button
          disabled={!isChange}
          class="save-button"
          on:click={handleSaveChanges}
        >
          {#if isLoading === true}
            <div class="loading-spinner"></div>
          {:else}
            Save Changes
          {/if}
        </button>
      {/if}
      {#if isSuccess === true}
        <div class="success_container">
          <img src="swarm/check_circle.svg" alt="success" class="" />
          <p class="success_text">Changes Saved</p>
        </div>
      {/if}
    </div>
  </div>
  <div class="content">
    <div class="about-container">
      <label class="graph-label" for="graph-title">Graph Title</label>
      <input
        id="graph-title"
        type="text"
        class="graph-input"
        bind:value={changedState.graph_name.value}
        on:input={handleInputChange}
      />
    </div>
    <div class="checkbox-content">
      <div class="checkbox-container">
        <input
          type="checkbox"
          class="checkbox"
          on:click={(e) => handleCheckBoxChange(e, "public")}
          checked={changedState.public.value}
        />
        <div class="checkout-label-container">
          <h4 class="checkout-label">Public</h4>
          <p class="checkout-label-description">
            Toggle to make the graph public or private.
          </p>
        </div>
      </div>
      {#each featureFlags as featureFlag}
        <div class="checkbox-container">
          <input
            type="checkbox"
            class="checkbox"
            on:click={(e) => handleCheckBoxChange(e, featureFlag.key)}
            checked={changedState[featureFlag.key].value}
          />
          <div class="checkout-label-container">
            <h4 class="checkout-label">{featureFlag.label}</h4>
            <p class="checkout-label-description">
              {featureFlag.description}
            </p>
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    padding-left: 2.25rem;
    padding-right: 2.25rem;
  }

  .header {
    display: flex;
    padding: 1.8125rem 0rem;
    align-items: center;
    justify-content: space-between;
  }

  .title {
    color: #fff;
    font-family: "Barlow";
    font-size: 1.125rem;
    font-style: normal;
    font-weight: 700;
    line-height: 1rem; /* 88.889% */
    letter-spacing: 0.01125rem;
  }

  .button-container {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 1rem;
  }

  .discard-button {
    display: flex;
    height: 2rem;
    padding: 0.75rem 1rem;
    justify-content: center;
    align-items: center;
    gap: 0.75rem;
    border-radius: 0.375rem;
    border: 1px solid rgba(107, 122, 141, 0.5);
    color: #fff;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 500;
    line-height: 1.1875rem; /* 146.154% */
    background-color: transparent;
    cursor: pointer;
  }

  .save-button {
    display: flex;
    height: 2rem;
    align-items: center;
    justify-content: center;
    border-radius: 0.375rem;
    padding: 0.75rem;
    gap: 0.375rem;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 146.154% */
    border: none;
    outline: none;
    background: #2fbe88;
    color: #fff;
    width: 6.5rem;
    cursor: pointer;
  }

  .save-button:disabled {
    background: rgba(48, 51, 66, 0.5);
    /* color: #23252f; */
    color: #52566e;
    cursor: not-allowed;
  }

  .loading-spinner {
    border: 2px solid #f3f3f3; /* Light grey */
    border-top: 2px solid #2fbe88; /* Blue */
    border-radius: 50%;
    width: 1.125rem;
    height: 1.125rem;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .success_container {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    height: 1rem;
  }

  .success_text {
    color: #49c998;
    font-family: "Roboto";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 123.077% */
    letter-spacing: 0.00813rem;
  }

  .content {
    display: flex;
    flex-direction: column;
  }

  .about-container {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.375rem;
  }

  .graph-label {
    color: #6b7a8d;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.125rem; /* 138.462% */
    letter-spacing: 0.00813rem;
  }

  .graph-input {
    width: 100%;
    height: 2.5rem;
    padding: 0.75rem 0.625rem;
    gap: 0.625rem;
    border-radius: 0.375rem;
    background: rgba(0, 0, 0, 0.2);
    border: none;
    outline: none;
    color: #fff;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 114.286% */
    letter-spacing: 0.00875rem;
  }

  .checkbox-content {
    display: flex;
    flex-direction: column;
    margin-top: 1.38rem;
    gap: 1.38rem;
  }

  .checkbox-container {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
  }

  .checkbox {
    width: 0.9375rem;
    height: 0.9375rem;
    margin: 0;
  }

  .checkout-label-container {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.375rem;
  }

  .checkout-label {
    color: #fff;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 500;
    line-height: 0.5rem; /* 61.538% */
    letter-spacing: 0.00813rem;
  }

  .checkout-label-description {
    color: #909baa;
    font-family: "Barlow";
    font-size: 0.75rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.1875rem; /* 158.333% */
  }
</style>
