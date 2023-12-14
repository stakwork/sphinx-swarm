<script>
  export let host = "";
  let link = host ? `https://${host}` : "http://localhost:8444";
</script>

<div class="nav-wrapper">
  <div class="title">Boltwall URL:</div>
  <div class="spacer" />
  <div>{link}</div>
</div>

<style>
  .nav-wrapper {
    font-size: 1rem;
    padding: 0px 25px;
  }
  .title {
    margin-top: 1rem;
  }
  .spacer {
    height: 1rem;
  }
</style>
