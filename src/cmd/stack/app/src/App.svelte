<script lang="ts">
    import * as cheerio from "cheerio";

    import Svelvet from "svelvet";

    $: height = window.innerHeight - 65;
    $: width = window.innerWidth - 450;

    $: node = "";
    $: number = "";
    $: links = "";
    $: nodeName = "";
    let display = "none";

    const nodeCallback = (node) => {
        const htmlData = cheerio.load(node.data.html);
        nodeName = htmlData("p.node-text").text();
        display = "block";
    }

    const initialNodes = [
        {
            id: 1,
            position: { x: 250, y: 50 },
            data: { html: "" +
                    "<section class='node-html'>" +
                        "<img src='swarm/lnd.png' class='node-img' width='50%'></img>" +
                        "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>LND Node</p>" +
                    "</section>" },
            width: 102,
            height: 120,
            bgColor: "#D4A74E",
            borderRadius: 10,
            clickCallback: nodeCallback
        },
        {
            id: 2,
            position: { x: 500, y: 110 },
            data: { html: "" +
                    "<section class='node-html'>" +
                    "<img src='swarm/tribeserver.png' class='node-img' width='50%'></img>" +
                    "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Tribe Server</p>" +
                    "</section>" },
            width: 102,
            height: 125,
            bgColor: "#618AFF",
            borderRadius: 10,
            clickCallback: nodeCallback
        },
        {
            id: 3,
            position: { x: 750, y: 170 },
            data: { html: "" +
                    "<section class='node-html'>" +
                    "<img src='swarm/relay.png' class='node-img' width='50%'></img>" +
                    "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Relay</p>" +
                    "</section>" },
            width: 102,
            height: 125,
            bgColor: "#49C998",
            borderRadius: 10,
            clickCallback: nodeCallback
        },
        {
            id: 4,
            position: { x: 480, y: 330 },
            data: { html: "" +
                    "<section class='node-html'>" +
                    "<img src='swarm/proxyserver.png' class='node-img' width='50%'></img>" +
                    "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Proxy Server</p>" +
                    "</section>" },
            width: 102,
            height: 125,
            bgColor: "#FF6161",
            borderRadius: 10,
            clickCallback: nodeCallback
        },
        {
            id: 5,
            position: { x: 250, y: 490 },
            data: { html: "" +
                    "<section class='node-html'>" +
                    "<img src='swarm/mqttserver.png' class='node-img' width='50%'></img>" +
                    "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>MQTT Server</p>" +
                    "</section>" },
            width: 102,
            height: 125,
            bgColor: "#660066",
            borderRadius: 10,
            clickCallback: nodeCallback
        },
        {
            id: 6,
            position: { x: 450, y: 630 },
            data: { html: "" +
                    "<section class='node-html'>" +
                    "<img src='swarm/channel.png' class='node-img' width='50%'></img>" +
                    "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Lightning Channel</p>" +
                    "</section>" },
            width: 102,
            height: 125,
            bgColor: "#9D61FF",
            borderRadius: 10,
            clickCallback: nodeCallback
        },
        {
            id: 7,
            position: { x: 700, y: 500 },
            data: { html: "" +
                    "<section class='node-html'>" +
                    "<img src='swarm/memeserver.png' class='node-img' width='50%'></img>" +
                    "<p class='node-text' style='color: #FFF; font-size: 0.8rem; margin-top: 10px; font-weight: bold'>Meme Server</p>" +
                    "</section>" },
            width: 102,
            height: 125,
            bgColor: "#9D61FF",
            borderRadius: 10,
            clickCallback: nodeCallback
        }
    ];

    const initialEdges = [
        { id: "e1-2", source: 1, target: 2 },
        { id: "e2-3", source: 2, target: 3 },
        { id: "e3-4", source: 3, target: 4 },
        { id: "e4-5", source: 4, target: 5 },
        { id: "e5-6", source: 5, target: 6 },
        { id: "e6-7", source: 6, target: 7 },
    ];
</script>

<main>
    <header>
        <div class="lefty logo-wrap"><img class="logo" alt="Sphinx icon" src="swarm/logo.jpg"/></div>
    </header>
    <div class="body">
        <div class="container">
            <section class="node-form">
                <h3 class="node-title">{nodeName}</h3>
                <form class="content" style="display: {display}">
                    <label>Name</label>
                    <input type="text" bind:value={node} />
                    <label>E-mail</label>
                    <input type="text" bind:value={number} />
                    <label>Telephone</label>
                    <input type="text" bind:value={links} />
                </form>
            </section>
            <Svelvet nodes={initialNodes} edges={initialEdges} bgColor="#13181D" {width} {height}  movement={false}  />
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
        padding: 50px;
        border-radius: 10px;
        align-items: center;
        max-height: calc(100vh);
        margin-right: 0px;
        width: calc(28vw - 30px);
    }

    .node-form .node-title {
        font-size: 1.4rem;
        margin-bottom: 20px;
        font-weight: bold;
    }

    label {
        font-size: 1rem;
        margin-bottom: 15px;
        display: block;
    }

    input  {
        width: 100%;
        padding: 10px;
        margin-bottom: 15px;
        height: 45px;
        border-radius: 5px;
        background: transparent;
        border: none;
        box-shadow: 0 1.5px 3px 0 rgba(211, 211, 211, 0.2), 0 2px 8px 0 rgba(211, 211, 211, 0.19);
    }

    .node-html {
        display: flex;
        flex-direction: column;
        align-items: center;
    }
</style>
