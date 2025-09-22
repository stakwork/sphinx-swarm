<script lang="ts">
  export let value: string;
</script>

<div class="key-card">
  <pre class="key-text">{value}</pre>
</div>

<style>
  .key-card {
    max-width: 32rem;
    border: 1px solid #ddd;
    border-radius: 0.5rem;
    background: #fff;
    padding: 0.75rem;
    margin: 0.5rem 0;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08);
  }

  .key-text {
    font-family: monospace;
    font-size: 0.875rem;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
    padding: 0.5rem;
    background: #f6f6f6;
    border-radius: 0.25rem;
  }
</style>
