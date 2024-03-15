<!-- Select.svelte -->
<script>
  export let options = [];
  export let value = "";
  export let label = "";
  export let valueChange;

  function handleChange(event) {
    value = event.target.value;
    valueChange(value);
  }
</script>

<div class="container">
  <label for={label}>{label}</label>
  <div class="select_container">
    <select id="" bind:value on:change={handleChange} class="custom-select">
      {#each options as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
    <img src="swarm/caret_down.svg" alt="caret down" class="caret_down" />
  </div>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    gap: 0.38rem;
  }

  label {
    color: #6b7a8d;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.125rem; /* 138.462% */
    letter-spacing: 0.00813rem;
  }

  .select_container {
    position: relative;
  }

  .custom-select {
    appearance: none;
    padding: 0.75rem 0.625rem 0.75rem 1rem;
    font-size: 16px;
    color: #fff;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 114.286% */
    letter-spacing: 0.00875rem;
    width: 100%;
    cursor: pointer;
    border-radius: 0.375rem;
    background: #303342;
    border: none;
    outline: none;
  }

  .custom-select option {
    padding: 8px 16px;
    font-size: 16px;
  }

  .caret_down {
    position: absolute;
    top: 50%;
    right: 0.62rem;
    transform: translateY(-50%);
    pointer-events: none; /* Ensures the icon doesn't interfere with select clicks */
    height: 1rem;
    width: 1rem;
  }
</style>
