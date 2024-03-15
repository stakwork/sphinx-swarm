<script lang="ts">
  export let isOpen;
  export let onClose;

  function closeBody(e) {
    onClose();
  }

  function preventPropagation(e) {
    e.stopPropagation();
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
  class="modal"
  style={isOpen ? "display: flex;" : "display: none;"}
  on:click={closeBody}
>
  <div class="modal-content" on:click={preventPropagation}>
    <slot />
  </div>
</div>

<style>
  .modal {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5); /* semi-transparent background overlay */
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 999; /* Ensure modal is above other content */
    cursor: pointer;
  }

  .modal-content {
    border-radius: 0.75rem;
    background-color: #23252f;
    max-width: 80%;
    max-height: 80%;
    overflow: auto; /* Enable scrolling if content exceeds modal size */
    cursor: default;
  }
</style>
