<script lang="ts">
  import {
    get_graph_accessibility,
    update_graph_accessibility,
  } from "../../api/swarm";
  import { onMount } from "svelte";

  $: isChange = false;
  $: isLoading = false;
  $: state = {
    graph_name: { value: "", method: async () => test() },
    trendingTopics: {
      value: true,
      method: async () => test(),
    },
    public: {
      value: true,
      method: async (value: boolean) => toggleGraphStatus(value),
    },
  };

  $: changedState = {
    graph_name: { value: "", isChange: false },
    trendingTopis: { value: true, isChange: false },
    public: { value: true, isChange: false },
  };

  function test() {}
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
    for (let key in changedState) {
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

  //update trending topics

  // TODO!!
  // Handle on mount to update state from boltwall
  // handle sending all change to the backend
  onMount(async () => {
    // get about details
    // get trendingTopics feature flag state

    // get public graph status
    const result = await get_graph_accessibility();
    const parsedResult = JSON.parse(result);
    console.log(parsedResult);

    //update state
    //update changedState
  });
</script>

<div class="container">
  <div class="header">
    <h2 class="title">General</h2>
    <div class="button-container">
      {#if isLoading === false && isChange === true}
        <button class="discard-button">Discard</button>
      {/if}
      <button disabled={!isChange} class="save-button">
        {#if isLoading === true}
          <div class="loading-spinner"></div>
        {:else}
          Save Changes
        {/if}
      </button>
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
      <div class="checkbox-container">
        <input
          type="checkbox"
          class="checkbox"
          on:click={(e) => handleCheckBoxChange(e, "trendingTopics")}
          checked={changedState.trendingTopis.value}
        />
        <div class="checkout-label-container">
          <h4 class="checkout-label">Trending Topics</h4>
          <p class="checkout-label-description">
            Toggle to display Trending topics on the graph.
          </p>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
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
    color: #23252f;
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
