<script>
    import Delete from "carbon-icons-svelte/lib/TrashCan.svelte";
</script>

<div class="tribes-wrap">
  <section class="header-wrap">
    <h1 class="default-header">Default Tribes</h1> <small>(A maximum of 5 default tribes)</small>
  </section>
  <div class="divider" />

  <div class="tribes-data">
    <section class="tribes">
        <p class="name">
            Sphinx dev
        </p>
        <button class="delete-btn"><Delete size={24} /></button>
    </section>
    <section class="tribes">
        <p class="name">
            Testing Sphinx
        </p>
        <button class="delete-btn"><Delete size={24} /></button>
    </section>
    <section class="tribes">
        <p class="name">
            Nostr
        </p>
        <button class="delete-btn"><Delete size={24} /></button>
    </section>
  </div>
</div>

<style>
  .tribes-wrap {
    padding: 1.5rem;
  }
  .header-wrap {
    display: flex;
    flex-direction: row;
    align-items: center;
  }

  .header-wrap
  .default-header {
    font-size: 1rem;
    font-weight: 400;
  }
  .header-wrap small {
    font-size: 0.65rem;
    color: #C6c6c6;
    margin-left: 15px;
  }
  .tribes-data .tribes {
    display: flex;
    flex-direction: row;
    align-items: center;
    padding: 0.5rem 0rem;
  }
  .tribes-data .tribes .name {
    font-size: 0.92rem;
    padding: 0;
    margin: 0;
  }
  .tribes-data .tribes .delete-btn {
    margin: 0;
    margin-left: auto;
    background-color: transparent;
    color: red;
    padding: 0;
    border: 0;
  }
</style>
