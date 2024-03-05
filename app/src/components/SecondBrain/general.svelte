<script lang="ts">
</script>

<div class="container">
  <div class="header">
    <h2 class="title">General</h2>
    <button disabled={true} class="save-button">Save Changes</button>
  </div>
  <div class="content">
    <div class="about-container">
      <label class="graph-label" for="graph-title">Graph Title</label>
      <input id="graph-title" type="text" class="graph-input" />
    </div>
    <div class="checkbox-content">
      <div class="checkbox-container">
        <input type="checkbox" class="checkbox" />
        <div class="checkout-label-container">
          <h4 class="checkout-label">Public</h4>
          <p class="checkout-label-description">
            Toggle to make the graph public or private.
          </p>
        </div>
      </div>
      <div class="checkbox-container">
        <input type="checkbox" class="checkbox" />
        <div class="checkout-label-container">
          <h4 class="checkout-label">Trending Topics</h4>
          <p class="checkout-label-description">
            Toggle to display Trending topics on the graph.
          </p>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    padding: 1.8125rem 0rem;
    align-items: center;
    justify-content: space-between;
  }

  .title {
    color: #fff;
    font-family: "Barlow";
    font-size: 1.125rem;
    font-style: normal;
    font-weight: 700;
    line-height: 1rem; /* 88.889% */
    letter-spacing: 0.01125rem;
  }

  .save-button {
    border-radius: 0.375rem;
    background: rgba(48, 51, 66, 0.5);
    padding: 0.75rem;
    gap: 0.375rem;
    color: #23252f;
    text-align: center;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 600;
    line-height: 1.1875rem; /* 146.154% */
    border: none;
    outline: none;
  }

  .content {
    display: flex;
    flex-direction: column;
  }

  .about-container {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.375rem;
  }

  .graph-label {
    color: #6b7a8d;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.125rem; /* 138.462% */
    letter-spacing: 0.00813rem;
  }

  .graph-input {
    width: 100%;
    height: 2.5rem;
    padding: 0.75rem 0.625rem;
    gap: 0.625rem;
    border-radius: 0.375rem;
    background: rgba(0, 0, 0, 0.2);
    border: none;
    outline: none;
    color: #fff;
    font-family: "Barlow";
    font-size: 0.875rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1rem; /* 114.286% */
    letter-spacing: 0.00875rem;
  }

  .checkbox-content {
    display: flex;
    flex-direction: column;
    margin-top: 1.38rem;
    gap: 1.38rem;
  }

  .checkbox-container {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
  }

  .checkbox {
    width: 0.9375rem;
    height: 0.9375rem;
  }

  .checkout-label-container {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.375rem;
  }

  .checkout-label {
    color: #fff;
    font-family: "Barlow";
    font-size: 0.8125rem;
    font-style: normal;
    font-weight: 500;
    line-height: 0.5rem; /* 61.538% */
    letter-spacing: 0.00813rem;
  }

  .checkout-label-description {
    color: #909baa;
    font-family: "Barlow";
    font-size: 0.75rem;
    font-style: normal;
    font-weight: 400;
    line-height: 1.1875rem; /* 158.333% */
  }
</style>
