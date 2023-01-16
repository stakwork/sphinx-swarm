<script>
  export let content = "";
  export let style = "";
</script>

<div on:click {style}>{content}</div>

<style>
  div {
    border-radius: 100%;
    cursor: pointer;
    height: 1.5rem;
    width: 1.5rem;
    border: 1px solid white;
    color: white;
    background: rgb(34, 34, 54);
    font-weight: bold;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 8px;
  }
  div:hover {
    background: white;
    color: black;
  }
</style>
