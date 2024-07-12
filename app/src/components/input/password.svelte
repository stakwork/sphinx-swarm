<script lang="ts">
  export let value = "";
  export let placeholder = "Enter text";
  export let onInput;
  export let label;
  export let readonly = false;

  $: hide = true;

  function handleInput(event) {
    const inputValue = event.target.value;
    onInput(inputValue);
  }

  function toggleHide() {
    hide = !hide;
  }
</script>

<div class="container">
  <label for={label} class="label">{label}</label>
  <div class="input_container">
    {#if hide}
      <input
        type="password"
        id={label}
        bind:value
        class="input"
        {placeholder}
        on:input={handleInput}
        {readonly}
      />
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <img
        src="swarm/hide.svg"
        alt="visibility"
        on:click={toggleHide}
        class="toggle"
      />
    {:else}
      <input
        type="text"
        id={label}
        bind:value
        class="input"
        {placeholder}
        on:input={handleInput}
        {readonly}
      />
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <img
        src="swarm/show.svg"
        alt="visibility"
        on:click={toggleHide}
        class="toggle"
      />
    {/if}
  </div>
</div>

<style>
  .container {
    display: flex;
    width: 100%;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .label {
    color: #909baa;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 500;
    line-height: 1.125rem; /* 138.462% */
    letter-spacing: 0.00813rem;
  }

  .input_container {
    width: 100%;
    background: #13181d;
    padding: 0.94rem;
    border-radius: 0.375rem;
    display: flex;
    justify-content: space-between;
  }

  .input {
    color: #ffffff;
    font-family: "Roboto";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 114.286% */
    letter-spacing: 0.00875rem;
    background: transparent;
    width: 100%;
    border: none;
    outline: none;
  }

  .input::placeholder {
    color: #556171;
    font-family: "Roboto";
    font-size: 0.875rem;
  }
  input:-webkit-autofill,
  input:-webkit-autofill:hover,
  input:-webkit-autofill:focus,
  input:-webkit-autofill:active {
    -webkit-box-shadow: 0 0 0 30px #13181d inset !important;
    caret-color: white;
  }

  input:-webkit-autofill {
    -webkit-text-fill-color: white !important;
  }

  .toggle {
    cursor: pointer;
  }
</style>
