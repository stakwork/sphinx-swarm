<script>
    import { Router, Route } from "svelte-routing";
    import Home from "./routes/Home.svelte";
    import UsersList from "./routes/lnd/UsersList.svelte";
  
    // Used for SSR. A falsy value is ignored by the Router.
    export let url = "";
  </script>
  
  <Router url="{url}">
    <div>
      <Route path="/" component="{Home}" />
      <Route path="/lnd/users" component="{UsersList}" />
    </div>
  </Router>
