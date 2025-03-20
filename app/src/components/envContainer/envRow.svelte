<script>
  import Input from "../input/input.svelte";
  export let value;
  export let key;
</script>

<div class="container">
  <div class="keyContainer"><p class="key">{key}</p></div>
  <Input label="" {value} readonly={true} onInput={() => {}} />
</div>

<style>
  .container {
    display: flex;
    align-items: center;
    gap: 2rem;
    width: 100%;
  }

  .key {
    font-weight: 500;
    font-size: 1rem;
    font-family: "Barlow";
  }

  .keyContainer {
    width: 32rem;
  }
</style>
