<script>
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import Launch from "carbon-icons-svelte/lib/Launch.svelte";

  export let select = () => {};
  export let name = "";
  export let preview = false;
  export let img = "";
  export let price_per_message = 0;
  export let selected = false;
  export let unique_name = "";
  export let member_count = 0;
  export let uuid = "";
  export let url = "";

  const defaultImage =
    "https://memes.sphinx.chat/public/HoQTHP3oOn0NAXOTqJEWb6HCtxIyN_14WGgiIgXpxWI=";
  function mainSelect() {
    if (!selected) select(uuid);
  }

  function back() {
    select(null);
  }
</script>

<div
  class={`tribe ${selected && "selected"}`}
  on:click={mainSelect}
  on:keypress={() => {}}
>
  {#if selected}
    <section>
      <div class="top">
        <div class="top-left">
          <div class="back" on:click={back} on:keypress={() => {}}>
            <ArrowLeft size={24} />
          </div>
          <h6>Tribe users {member_count}</h6>
        </div>
        {#if selected}
          <a
            href={`https://${url}/t/${unique_name}`}
            class="tribe-link"
            target="_blank"
            rel="noreferrer"><Launch size={24} /></a
          >
        {/if}
      </div>
      <div class="message-price">
        Price per message: {`${price_per_message} sats`}
      </div>
    </section>
  {:else}
    <section class="tribedata-wrap">
      <img src={`${img || defaultImage}`} alt="Tribe logo" class="tribe-logo" />
      <div class="name">{name || "Tribe"}</div>

      {#if preview}
        <a
          href={`https://cache.sphinx.chat/?tribe=${uuid}`}
          class="preview-link"
          target="_blank"
          rel="noreferrer">Preview</a
        >
      {/if}
    </section>
  {/if}
</div>

<style>
  .tribe {
    font-size: 1rem;
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 0.8rem 1.21rem;
    cursor: pointer;
  }
  .tribe:not(.selected) {
    max-height: 75px;
  }
  .tribe:hover:not(.selected) {
    background: #131b23;
  }
  .tribe-logo {
    width: 45px;
    height: 45px;
    border-radius: 50%;
    margin-right: 10px;
  }
  .tribedata-wrap {
    display: flex;
    align-items: center;
    margin-bottom: 8px;
  }
  .user:not(.selected) {
    cursor: pointer;
  }
  .user:hover:not(.selected) {
    background: #131b23;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .back {
    cursor: pointer;
    margin-left: 0.1rem;
    height: 2rem;
    width: 2rem;
    border-radius: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .back:hover {
    background: #070b0e;
  }
  .top-left {
    display: flex;
    align-items: center;
    width: 100%;
  }
  .message-price {
    margin-left: 0.1rem;
    font-size: 0.7rem;
    color: gray;
  }
  .fields {
    padding: 1.5rem;
  }
  .user-values-title {
    margin: 10px 0px;
    color: grey;
    font-size: 0.78rem;
    font-weight: bold;
  }
  .value-wrap {
    display: flex;
    align-items: center;
  }
  .user-value {
    max-width: 97%;
    color: #fefefe;
    font-size: 0.7rem;
    color: #fefefe;
    overflow-x: scroll;
    white-space: nowrap;
    margin-right: 1.2%;
    padding-bottom: 11px;
  }
  .empty-alias {
    height: 0.85rem;
    width: 6rem;
    background: #263442;
  }
  .preview-link {
    text-decoration: none;
    margin-left: auto;
    font-size: 0.9rem;
    color: #ddd;
  }
  .tribe-link {
    margin-right: 1.25rem;
  }
</style>
