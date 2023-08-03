<script lang="ts">
  import { Button, Loading } from "carbon-components-svelte";
  import Logs from "carbon-icons-svelte/lib/CloudLogging.svelte";
  import ArrowLeft from "carbon-icons-svelte/lib/ArrowLeft.svelte";
  import * as api from "../api";
  import { onDestroy } from "svelte";
  import {get_container_stat} from "../api/swarm";
  import { Doughnut } from 'svelte-chartjs';
  import {
      Chart as ChartJS,
      Title,
      Tooltip,
      Legend,
      ArcElement,
      CategoryScale,
  } from 'chart.js';
  import {selectedNode} from "../store";

  ChartJS.register(Title, Tooltip, Legend, ArcElement, CategoryScale);

  let open = false;
  export let nodeName = "";
  let default_stats = {
    container_name: "",
    cpu_total_usage: 0,
    system_cpu_usage: 0,
    memory_usage: 0,
  };
  let stats = [{...default_stats}];
  let data = getData();
  let isLoading = false;



  async function getNodeStats() {
    open = true;
    isLoading =  true;
    stats = [{...default_stats}];
    data = []
    let theStats = []
    if (!$selectedNode){
      theStats = await api.swarm.get_container_stat(``);
      if (theStats.length > 0){
        stats =[]
        for (let i = 0; i < theStats.length; i++) {
          stats.push({
            container_name: theStats[i].container_name,
            cpu_total_usage: theStats[i].cpu_total_usage,
            system_cpu_usage: theStats[i].system_cpu_usage,
            memory_usage: theStats[i].memory_usage,
          })
        }
      }
    } else {
      theStats = await api.swarm.get_container_stat(`${$selectedNode?.name}.sphinx`);
      stats = [{
        container_name: theStats[0].container_name,
        cpu_total_usage: theStats[0].cpu_total_usage,
        system_cpu_usage: theStats[0].system_cpu_usage,
        memory_usage: theStats[0].memory_usage,
      }];
    }
    console.log(">>>>>> ", theStats)

    data = getData();
    isLoading =  false;

    // if (theLogs) logs = theLogs.reverse();
  }

  function getData() {
    let doughnut_data = []
    console.log(">>>>>>isGlobalStats  ", $selectedNode)
    if(!$selectedNode){
      for (let i = 0; i < stats.length; i++) {
        doughnut_data.push({
          container_name: stats[i].container_name,
          labels: ['System CPU Usage', 'Memory Usage', 'CPU Total Usage'],
          datasets: [
            {
              data: [stats[i].system_cpu_usage, stats[i].memory_usage, stats[i].cpu_total_usage],
              // data: [stats.system_cpu_usage, 5324535, 1109028864],
              backgroundColor: ['#F7464A', '#46BFBD', '#FDB45C'],
              hoverBackgroundColor: [
                '#FF5A5E',
                '#5AD3D1',
                '#FFC870',
              ],
            },
          ],
        });
      }
    } else {
      doughnut_data.push({
        container_name: stats[0].container_name,
        labels: ['System CPU Usage', 'Memory Usage', 'CPU Total Usage'],
        datasets: [
          {
            data: [stats[0].system_cpu_usage, stats[0].memory_usage, stats[0].cpu_total_usage],
            // data: [stats.system_cpu_usage, 5324535, 1109028864],
            backgroundColor: ['#F7464A', '#46BFBD', '#FDB45C'],
            hoverBackgroundColor: [
              '#FF5A5E',
              '#5AD3D1',
              '#FFC870',
            ],
          },
        ],
      });
    }
    console.log("<<<<>>>>>> >>>>", doughnut_data[0])

    return doughnut_data;
  }

  onDestroy(() => {
    stats = [{...default_stats}];
  });
</script>

<!--<section class="get-logs-btn">-->
<section class="stats_section">
  <Button
          size="field" kind="secondary" icon={Logs} on:click={getNodeStats}
    >Get Stats</Button
  >

<!--  <Button-->
<!--          on:click={getNodeStats}-->
<!--          icon={Logs}-->
<!--          size="field"-->
<!--          kind="secondary"-->
<!--          class="get_stats_btn">Get Stats</Button-->
<!--  >-->

  <div class="modal" style={`display: ${open ? "block" : "none"}`}>
    <section class="modal-head">
      <button on:click={() => (open = !open)}>
        <ArrowLeft size={32} />
      </button>
      <h4 class="modal-title">{nodeName.toLocaleUpperCase()} Stats</h4>
    </section>
    {#if isLoading}
      <div class="loader">
        <Loading />
      </div>
    {/if}
    <section class="container">
      <div class="stats">
        {#if stats !== null}
          {#if stats.length === 1}
            <div class="stat">Container Name: {data[0]?.container_name || ''}</div>
            <div class="stat">System CPU Usage: {data[0]?.datasets[0].data[0] || 0}</div>
            <div class="stat">Memory Usage: {data[0]?.datasets[0].data[1] || 0}</div>
            <div class="stat">CPU Total Usage: {data[0]?.datasets[0].data[2] || 0}</div>
            <div class="doughnut"> <Doughnut data={data[0]} options={{ responsive: true, class: "log" }} /> </div>
          {:else}
            <div class="grid-container">
              {#each data as datum}
                <div class="doughnut-box">
                  <div class="stat">Container Name: {datum.container_name || ''}</div>
                  <div class="stat">System CPU Usage: {datum.datasets[0].data[0] || 0}</div>
                  <div class="stat">Memory Usage: {datum.datasets[0].data[1] || 0}</div>
                  <div class="stat">CPU Total Usage: {datum.datasets[0].data[2] || 0}</div>
                  <div class="mini-doughnut"> <Doughnut data={datum} options={{ responsive: true, class: "mini-doughnut" }} /> </div>
                </div>
              {/each}
            </div>
          {/if}
          <!--{:else if stats.length > 1}-->
        <!--  {#each stats as stat}-->
        <!--    <div class="mini-doughnut"> <Doughnut {data} options={{ responsive: true, class: "mini-doughnut" }} /> </div>-->
        <!--  {/each}-->
        <!--{:else}-->
        {/if}
      </div>
    </section>
  </div>
</section>

<style>
  .stats_section {
    margin-right: 1rem;
  }

  .get-logs-btn {
    margin-left: 20px;
  }
  .modal {
    height: 88vh;
    z-index: 100;
    width: 98vw;
    position: absolute;
    left: 1%;
    right: 1%;
    bottom: 2%;
    background: #1a242e;
    border: 0.8px solid white;
  }
  .modal-head {
    display: flex;
    align-items: center;
    padding-top: 1rem;
    padding-left: 2.5rem;
  }
  .modal-head button {
    padding: 0;
    background: 0;
    border: 0;
    cursor: pointer;
    color: #fff;
    font-weight: 900;
  }
  .modal-head .modal-title {
    padding: 0;
    margin: 0;
    margin-left: 20px;
    font-size: 1.2rem;
    font-weight: 600;
  }
  .modal-content {
    padding: 2rem 2.5rem;
    padding-top: 1.2rem;
  }
  .stats {
    background: #393939;
    width: 100%;
    min-height: 30vh;
    max-height: 76vh;
    overflow: auto;
    padding: 0.3rem 0.5rem;
    display: flex;
    flex-direction: column-reverse;
  }
  .stat {
    color: white;
    margin: 1px 0;
    font-size: 0.8rem;
  }
  .doughnut {
    max-height: 35rem;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .mini-doughnut {
    max-height: 10rem;
    max-width: 10rem;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .grid-container {
    display: flex;
    flex-wrap: wrap;
  }
  .doughnut-box {
    margin: 2rem;
    flex-wrap: wrap;
  }

</style>
