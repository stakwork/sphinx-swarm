<script>
    import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
    import { initialUsers } from "./users";
    import User from "./User.svelte";

    export let select = () => {};
    export let name = "";
    export let preview = "";
    export let logo = "";
    export let price_per_message = 0;
    export let selected = false;
    export let member_count = 0;
    export let uuid = "";

    let selectedPubkey = "";
    $: selectedUser = initialUsers.find((u) => u.pubkey === selectedPubkey);

    const defaultImage =
      "https://memes.sphinx.chat/public/HoQTHP3oOn0NAXOTqJEWb6HCtxIyN_14WGgiIgXpxWI=";
    function mainSelect() {
      if (!selected) select(name);
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
        </div>
  
        <div class="divider" />
        {#if selectedUser}
          <User
            {...selectedUser}
            selected={true}
            select={() => (selectedPubkey = null)}
          />
        {:else}
          {#each initialUsers as user}
            <User
              {...user}
              select={(pubkey) => (selectedPubkey = pubkey)}
              selected={false}
            />
          {/each}
        {/if}
      </section>
    {:else if name}
      <section class="tribedata-wrap">
        <img
          src={`${logo || defaultImage}`}
          alt="Tribe logo"
          class="tribe-logo"
        />
        <div class="name">{name}</div>
  
        {#if preview}
          <a href={`https://cache.sphinx.chat/?tribe=${uuid}`} class="preview-link" target="_blank" rel="noreferrer"
            >Preview</a
          >
        {/if}
      </section>
      <div class="message-price">
        Price per message: {`${price_per_message} sats`}
      </div>
    {:else}
      <div class="empty-alias" />
    {/if}
  </div>
  
  <style>
    .tribe {
      font-size: 1rem;
      position: relative;
      display: flex;
      flex-direction: column;
      padding: 0.8rem 1.21rem;
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
    .pubkey {
      color: grey;
      margin-bottom: 0.5rem;
      font-size: 0.7rem;
      max-width: 88%;
      text-overflow: ellipsis;
      overflow: hidden;
      white-space: nowrap;
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
  </style>