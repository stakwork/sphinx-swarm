<script>
  import * as api from "../api";

  async function startContainer(id) {
    const res = await api.swarm.start_container(id);
  }

  async function stopContainer(id) {
    const res = await api.swarm.stop_container(id);
  }
</script>

<aside class="node-action-wrap">
  <button class="btn btn-stop">Stop</button>
</aside>

<style>
  .node-action-wrap {
    margin-left: 20px;
  }
  .node-action-wrap .btn {
    padding: 6px 8px;
    border: 0px;
    outline: 0px;
    border-radius: 2px;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .node-action-wrap .btn-stop {
    background-color: red;
  }
  .node-action-wrap .btn-start {
    background-color: green;
  }
</style>
