<script>
</script>

<div>
  <section class="liquidity-wrap">
    <aside>
      <h6 class="title">TOTAL INBOUND LIQUIDITY</h6>
      <h3 class="value">34 945 934 <span>SAT</span></h3>
    </aside>
    <aside>
      <h6 class="title">TOTAL OUTBOUND LIQUIDITY</h6>
      <h3 class="value">24 045 934 <span>SAT</span></h3>
    </aside>
  </section>
  <section class="lnd-table-wrap">
    <table>
      <thead>
        <th></th>
        <th>CAN RECEIVE</th>
        <th>CAN SEND</th>
        <th>PEER / ALIAS</th>
      </thead>
      <tbody>
        <tr>
          <td>..</td>
          <td>2125000</td>
          <td>1125000</td>
          <td>OpenNode</td>
        </tr>
      </tbody>
    </table>
  </section>
</div>

<style>
  .liquidity-wrap {
    background-color: #101317;
    padding: 25px 30px;
    display: flex;
  }

  .liquidity-wrap aside {
    text-align: center;
    width: 50%;
  }

  .liquidity-wrap aside:first-child {
    border-right: 1px solid #6a6d6c;
  }

  .liquidity-wrap aside .title {
    font-size: 0.85rem;
    color: #6B7A8D;
  }

  .liquidity-wrap aside .value {
    font-size: 1.6rem;
    color: #ffffff;
    margin-top: 10px;
  }

  .liquidity-wrap aside .value span {
    color: #6B7A8D;
  }
</style>
