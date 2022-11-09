<script lang="ts">
    import * as cheerio from "cheerio";
    import Svelvet from "svelvet";
    import {
      NumberInput,
      TextInput,
      Grid,
      Row,
      Column,
      Button
    } from "carbon-components-svelte";
    import Add from "carbon-icons-svelte/lib/Add.svelte";
    import { Link } from "svelte-routing";
  
    $: height = window.innerHeight - 65;
    $: width = window.innerWidth - 420;
  
    $: nodeCount = 0;
    $: nodeName = "Default";
  
    const nodeCallback = (node) => {
      const htmlData = cheerio.load(node.data.html);
      nodeName = htmlData("p.node-text").text();
    };
  
    const initialNodes = [
      {
        id: 1,
        position: { x: 250, y: 100 },
        data: {
          html:
            "" +
            "<section class='node-html'>" +
            "<img src='swarm/lnd.png' class='node-img' width='50%'></img>" +
            "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>LND Node</p>" +
            "</section>",
        },
        width: 102,
        height: 120,
        bgColor: "#D4A74E",
        borderRadius: 10,
        clickCallback: nodeCallback,
      },
      {
        id: 2,
        position: { x: 600, y: 100 },
        data: {
          html:
            "" +
            "<section class='node-html'>" +
            "<img src='swarm/tribeserver.png' class='node-img' width='50%'></img>" +
            "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Tribe Server</p>" +
            "</section>",
        },
        width: 102,
        height: 125,
        bgColor: "#618AFF",
        borderRadius: 10,
        clickCallback: nodeCallback,
      },
      {
        id: 3,
        position: { x: 420, y: 270 },
        data: {
          html:
            "" +
            "<section class='node-html'>" +
            "<img src='swarm/relay.png' class='node-img' width='50%'></img>" +
            "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Relay</p>" +
            "</section>",
        },
        width: 102,
        height: 125,
        bgColor: "#49C998",
        borderRadius: 10,
        clickCallback: nodeCallback,
      },
      {
        id: 4,
        position: { x: 350, y: 550 },
        data: {
          html:
            "" +
            "<section class='node-html'>" +
            "<img src='swarm/proxyserver.png' class='node-img' width='50%'></img>" +
            "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Proxy Server</p>" +
            "</section>",
        },
        width: 102,
        height: 125,
        bgColor: "#9D61FF",
        borderRadius: 10,
        clickCallback: nodeCallback,
      },
      {
        id: 5,
        position: { x: 700, y: 470 },
        data: {
          html:
            "" +
            "<section class='node-html'>" +
            "<img src='swarm/memeserver.png' class='node-img' width='50%'></img>" +
            "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Meme Server</p>" +
            "</section>",
        },
        width: 102,
        height: 125,
        bgColor: "#FF6161",
        borderRadius: 10,
        clickCallback: nodeCallback,
      },
      {
        id: 6,
        position: { x: 950, y: 700 },
        data: {
          html:
            "" +
            "<section class='node-html'>" +
            "<img src='swarm/mqttserver.png' class='node-img' width='50%'></img>" +
            "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>MQTT Server</p>" +
            "</section>",
        },
        width: 102,
        height: 125,
        bgColor: "#660066",
        borderRadius: 10,
        clickCallback: nodeCallback,
      },
      {
        id: 7,
        position: { x: 950, y: 400 },
        data: {
          html:
            "" +
            "<section class='node-html'>" +
            "<img src='swarm/channel.png' class='node-img' width='50%'></img>" +
            "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Lightning Channel</p>" +
            "</section>",
        },
        width: 102,
        height: 125,
        bgColor: "#9D61FF",
        borderRadius: 10,
        clickCallback: nodeCallback,
      },
    ];
  
    const initialEdges = [
      { id: "e3-1", source: 3, target: 1, noHandle: true, type: "straight" },
      { id: "e3-2", source: 3, target: 2, noHandle: true, type: "straight" },
      { id: "e3-4", source: 3, target: 4, noHandle: true, type: "straight" },
      { id: "e4-5", source: 4, target: 5, noHandle: true, type: "straight" },
      { id: "e5-6", source: 5, target: 6, noHandle: true, type: "straight", animate: true },
      { id: "e6-7", source: 5, target: 7, noHandle: true, type: "straight", animate: true },
    ];
  </script>
  
  <main>
    <header>
      <div class="lefty logo-wrap">
        <img class="logo" alt="Sphinx icon" src="swarm/logo.jpg" />
      </div>
    </header>
    <div class="body">
      <div class="container">
        <Grid>
          <Row>
            <Column md={4} lg={4}>
              <section class="node-form">
                {#if nodeName === "LND Node"}
                  <section class="title-wrap">
                    <h3 class="node-title">{nodeName}</h3>
                    <h4 class="node-count"><Link to="/lnd/users">(42)</Link></h4>
                    <Button class="add-btn" icon={Add}>Add</Button>
                  </section>
                {:else}
                  <section class="title-wrap">
                    <h3 class="node-title">{nodeName}</h3>
                  </section>
                {/if}
  
                <form class="content">
                  <div class="controls">
                    <NumberInput label={"Count"} value={nodeCount} />
                    <div class="spacer" />
                    <TextInput labelText={"Name"} value={nodeName} />
                    <div class="spacer" />
                    <TextInput labelText={"Name"} value={nodeName} />
                  </div>
                </form>
              </section>
            </Column>
            <Column md={8} lg={8}>
              <Svelvet
                nodes={initialNodes}
                edges={initialEdges}
                bgColor="#13181D"
                {width}
                {height}
                movement={false}
              />
            </Column>
          </Row>
        </Grid>
      </div>
    </div>
  </main>
  
  <style>
    main {
      height: 100vh;
      width: 100vw;
      display: flex;
      background: rgb(20, 29, 39);
      flex-direction: column;
    }
  
    header {
      height: 4.2rem;
      display: flex;
      align-items: center;
      border-bottom: 1px solid rgba(211, 211, 211, 0.2);
    }
  
    .logo-wrap {
      display: flex;
      align-items: center;
    }
  
    .logo-wrap .logo {
      width: 70px;
      padding: 12px;
      margin-left: 2.5rem;
    }
  
    .body {
      display: flex;
      height: 100%;
    }
  
    .lefty {
      width: 15rem;
      max-width: 15rem;
      height: 100%;
      /* border-right: 1px dashed #bfbfbf; */
    }
  
    .node-form {
      padding: 15px;
      align-items: center;
      max-height: calc(100vh);
      margin-right: 0px;
    }
  
    .node-form .node-title {
      font-size: 1.4rem;
      font-weight: bold;
      padding: 0px;
    }

    .node-form .node-count {
      font-size: 1.4rem;
      font-weight: bold;
      padding: 0px;
      margin-left: 10px;
      margin-right: auto;
    }
  
    .spacer {
      margin-bottom: 1rem;
    }
  
    .title-wrap {
      display: flex;
      flex-direction: row;
      margin: 20px 0px;
      align-items: center;
    }
  </style>