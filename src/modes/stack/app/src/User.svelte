<script>
  import Login from "carbon-icons-svelte/lib/Login.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import CopyIcon from "carbon-icons-svelte/lib/Copy.svelte";
  import QrCode from "svelte-qrcode";

  export let select = () => {};
  export let alias = "";
  export let pubkey = "";
  export let routeHint = "";
  export let balance = 0;
  export let selected = false;

  const signedUp = alias ? true : false;

  function mainSelect() {
    if (!selected) select(pubkey);
  }
  function back() {
    select(null);
  }
  function copyToClipboard(value) {
    navigator.clipboard.writeText(value);
    // alert(value);
  }
</script>

<div
  class={`user ${selected && "selected"}`}
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
      <div class="dot-wrap">
        <div
          class="dot"
          style={`background:${signedUp ? "#52B550" : "grey"};`}
        />
      </div>
      {#if alias}
        <div class="alias">{alias}</div>
      {:else}
        <div class="empty-alias" />
      {/if}
    </div>
    <div class="signed-up" style={`opacity:${signedUp ? 1 : "0.5"}`}>
      <Login size={12} />
      <span>Signed Up</span>
    </div>
  </div>
  {#if selected}
    <div class="fields">
      <p class="user-values-title">Pubkey</p>
      <section class="value-wrap">
        <p class="user-value" >{pubkey}</p>
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <span on:click={copyToClipboard(pubkey)}><CopyIcon size={0} class="copy-icon" /></span>
      </section>
      {#if routeHint}
        <p class="user-values-title">Route hint</p>
        <section class="value-wrap">
          <p class="user-value">{routeHint}</p>
          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <span on:click={copyToClipboard(routeHint)}><CopyIcon size={0} class="copy-icon" /></span>
        </section>
      {/if}
      <p class="user-values-title">Invite QR code</p>
      <QrCode padding={1.5} value={pubkey} />
    </div>
  {:else}
    <div class="pubkey collapsed">
      {pubkey}
    </div>
    <div class="balance collapsed">
      {`${balance} sats`}
    </div>
  {/if}
</div>

<style>
  .user {
    font-size: 1rem;
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 0.8rem 0;
  }
  .user:not(.selected) {
    cursor: pointer;
  }
  .user:hover:not(.selected) {
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
  .signed-up {
    display: flex;
    align-items: center;
    text-transform: uppercase;
    font-size: 0.6rem;
    margin-right: 1rem;
  }
  .signed-up span {
    margin-left: 0.2rem;
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

  .balance {
    font-size: 0.8rem;
    color: #ddd;
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
  .collapsed {
    margin-left: 2.3rem;
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
</style>
