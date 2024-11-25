<script lang="ts">
  import { splitPubkey } from "../../helpers/swarm";

  export let value = "";
  export let placeholder = "Enter text";
  export let onInput;
  export let label;
  export let isPubkey = false;

  function handleInput(event) {
    let value = event.target.value;
    if (isPubkey && value.length > 66) {
      value = splitPubkey(value);
    }

    onInput(value);
  }
</script>

<div class="container">
  <label for={label} class="label">{label}</label>
  <input
    id={label}
    bind:value
    class="input"
    {placeholder}
    on:input={handleInput}
  />
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

  .input {
    padding: 0.94rem;
    border-radius: 0.375rem;
    background: #13181d;
    color: #ffffff;
    font-family: "Roboto";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 114.286% */
    letter-spacing: 0.00875rem;
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
</style>
