<script>
  export let label = "";
  export let cmd = "";

  function copy() {
    navigator.clipboard.writeText(cmd);
  }
</script>

<div class="cmd">
  <svg class="copy" on:click={copy} viewBox="0 0 24 24">
    <path
      d="M22 6v16h-16v-16h16zm2-2h-20v20h20v-20zm-24 17v-21h21v2h-19v19h-2z"
    />
  </svg>
  {#if label}<span>{label}</span>{/if}
  <span>{cmd}</span>
</div>

<style>
  .copy {
    fill: #bbb;
    height: 10px;
    margin-right: 0.2rem;
    cursor: pointer;
  }
  .copy:hover {
    fill: white;
  }
  .cmd {
    margin: 0.25rem 0;
    color: #7f7f7f;
    font-size: 13px;
    margin-bottom: 0.5rem;
  }
  .cmd span:last-child {
    font-weight: bold;
    color: white;
    margin-left: 5px;
  }
</style>
