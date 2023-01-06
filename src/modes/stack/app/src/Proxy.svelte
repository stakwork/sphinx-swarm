<script>
    import {formatSatsNumbers} from "./helpers";
</script>

<div class="proxy-wrapper">
  <h5 class="info">Proxy Stats</h5>
  <div class="spacer" />
    <section class="value-wrap">
      <h3 class="title">TOTAL USERS</h3>
      <h3 class="value">100</h3>
    </section>
    <section class="value-wrap">
      <h3 class="title">TOTAL SATS BALANCE</h3>
      <h3 class="value">{formatSatsNumbers(50000000)}</h3>
    </section>
</div>

<style>
  .proxy-wrapper {
    padding: 0px 25px;
  }
  .proxy-wrapper .info {
    font-size: 1rem;
    margin-bottom: 30px;
  }
  .value-wrap {
    display: flex;
    align-items: center;
    margin-bottom: 20px;
  }
  .title {
    color: #c6c6c6;
    font-size: 0.75rem;
  }
  .value {
    font-size: 0.85rem;
    margin-left: auto;
    font-weight: 600;
  }
</style>
