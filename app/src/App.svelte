<script lang="ts">
  import { rez, send_cmd, logs } from "./api";

  function send(txt: string) {
    if (txt === "clear\n") {
      return rez.set([]);
    }
    rez.update((r) => [`$ ${txt}`, ...r]);
    send_cmd(txt);
  }

  let txt = "";
  function keypress(e) {
    if (e.key === "Enter") {
      send(txt);
      txt = "";
      e.stopPropagation();
      setTimeout(() => {
        e.target.setSelectionRange(0, 0);
      }, 1);
    }
  }
</script>

<section>
  <h5>Core Lightning Logs</h5>
  <p>
    {#each $logs as log}
      <div class="log">{log}</div>
    {/each}
  </p>
</section>
<section>
  <h5>Terminal</h5>
  <p>
    {#each $rez as cmd}
      <div class="log">{cmd}</div>
    {/each}
  </p>
  <div class="txt-wrap">
    <textarea
      autofocus
      bind:value={txt}
      on:keypress={keypress}
      placeholder="Type commands here"
    />
    <span>$</span>
  </div>
</section>

<style>
  section {
    height: 100vh;
    width: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    margin: 0 0.4rem;
  }
  h5 {
    color: grey;
    text-transform: uppercase;
    font-size: 1em;
    font-weight: 200;
    margin: 0;
    width: 100%;
    margin-top: 5px;
  }
  textarea {
    margin-left: 18px;
    border: none;
    width: calc(100% - 20px);
    height: 100%;
    overflow: scroll;
    display: inline-block;
    outline: none;
    color: white;
    font-family: Courier, sans-serif;
    font-size: 17px;
    resize: none;
    background: rgb(34, 34, 54);
  }
  p {
    width: 100%;
    height: 100%;
    max-height: 100%;
    overflow-y: auto;
    border: 1px solid white;
    background: rgb(34, 34, 54);
    color: #ddd;
    font-family: Courier, sans-serif;
    font-size: 14px;
    display: flex;
    flex-direction: column-reverse;
    align-items: center;
    margin-top: 0.5rem;
  }
  .log {
    text-align: left;
    padding: 2px 5px;
    width: 100%;
    font-size: 14px;
    margin: 1px 0;
  }
  .txt-wrap {
    border: 1px solid white;
    border-radius: 2px;
    margin-bottom: 0.8rem;
    width: 100%;
    height: 100%;
    max-height: 100px;
    position: relative;
    background: rgb(34, 34, 54);
  }
  .txt-wrap span {
    position: absolute;
    left: 8px;
    top: 7px;
    color: white;
  }
</style>
