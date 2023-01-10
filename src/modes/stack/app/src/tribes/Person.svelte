<script lang="ts">
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import CopyIcon from "carbon-icons-svelte/lib/Copy.svelte";
  import Launch from "carbon-icons-svelte/lib/Launch.svelte";

  export let select = (pubkey: string) => {};
  export let owner_alias = "";
  export let owner_pubkey = "";
  export let owner_route_hint = "";
  export let img = "";
  export let selected = false;
  export let url = "";

  let peopleUrlArr = url.split(".");
  peopleUrlArr.shift();
  let peopleUrl = `people.${peopleUrlArr.join(".")}`;

  const defaultImage =
    "https://memes.sphinx.chat/public/HoQTHP3oOn0NAXOTqJEWb6HCtxIyN_14WGgiIgXpxWI=";

  function mainSelect() {
    if (!selected) select(owner_pubkey);
  }
  function back() {
    select(null);
  }
  function copyToClipboard(value) {
    navigator.clipboard.writeText(value);
  }
</script>

<div
  class={`person ${selected && "selected"}`}
  on:click={mainSelect}
  on:keypress={() => {}}
>
  <div class="top">
    <div class="top-left">
      {#if selected}
        <div class="back" on:click={back} on:keypress={() => {}}>
          <ArrowLeft size={24} />
        </div>
      {/if}
      <img
        src={`${img || defaultImage}`}
        alt="Person logo"
        class="person-img"
      />
      {#if owner_alias}
        <div class="alias">{owner_alias}</div>
      {:else}
        <div class="empty-alias" />
      {/if}
    </div>
    {#if selected}
      <a
        href={`https://${peopleUrl}/p/${owner_pubkey}`}
        class="person-link"
        target="_blank"
        rel="noreferrer"><Launch size={24} /></a
      >
    {/if}
  </div>
  {#if selected}
    <div class="fields">
      <p class="user-values-title">Pubkey</p>
      <section class="value-wrap">
        <p class="user-value">{owner_pubkey}</p>
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <span on:click={() => copyToClipboard(owner_pubkey)}
          ><CopyIcon size={16} class="copy-icon" /></span
        >
      </section>
      {#if owner_route_hint}
        <p class="user-values-title">Route hint</p>
        <section class="value-wrap">
          <p class="user-value">{owner_route_hint}</p>
          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <span on:click={() => copyToClipboard(owner_route_hint)}
            ><CopyIcon size={16} class="copy-icon" /></span
          >
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .person {
    font-size: 1rem;
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 0.8rem 0;
  }
  .person:not(.selected) {
    cursor: pointer;
    max-height: 75px;
  }
  .person:hover:not(.selected) {
    background: #131b23;
  }
  .dot-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1rem;
    margin: 0 0.7rem;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 100%;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .back {
    cursor: pointer;
    margin-left: 1rem;
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
  .person-img {
    width: 45px;
    height: 45px;
    border-radius: 50%;
    margin: 0 1rem;
  }
  .person-link {
    margin-right: 1.25rem;
  }
</style>
