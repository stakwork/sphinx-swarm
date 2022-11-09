<script lang="ts">
    import {
      Grid,
      Row,
      Column,
    } from "carbon-components-svelte";
  </script>
  
  <main>
    <header>
      <div class="lefty logo-wrap">
        <img class="logo" alt="Sphinx icon" src="../swarm/logo.jpg" />
      </div>
    </header>
    <div class="body">
      <div class="container">
        <Grid>
          <Row>
            <Column md={12} lg={12}>
              <h1>Users Lists</h1>
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