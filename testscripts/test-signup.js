import fetch from "node-fetch";

const swarm_ip = "0.0.0.0:8000";

const ip = "localhost:3000";

async function testSignup() {
  try {
    let hasadmin = false;
    while (!hasadmin) {
      const doeshaveadmin = await checkAdmin();
      if (doeshaveadmin) hasadmin = true;
    }
    const pubkey = await initial_admin_pubkey();
    console.log("initial_admin_pubkey", pubkey);
    const signedup = await signup("RANDOM", pubkey);
    console.log("signedup", signedup);
    if (signedup.success) {
      console.log("====> admin setup successfully <======");
    } else {
      console.log("=====> An error occured <======");
    }
  } catch (error) {
    console.log("ERROR:", error);
  }
}

async function checkAdmin() {
  const res = await fetch(`http://${ip}/has_admin`, {
    method: "GET",
    headers: { "Content-Type": "application/json" },
  });
  const j = await res.json();
  return j.response || false;
}

async function initial_admin_pubkey() {
  const res = await fetch(`http://${ip}/initial_admin_pubkey`, {
    method: "GET",
    headers: { "Content-Type": "application/json" },
  });
  const j = await res.json();
  if (!j.success) {
    throw j.error;
  }
  console.log(j);
  return j.response.pubkey;
}

async function signup(token, pubkey) {
  try {
    const res = await fetch(`http://${ip}/contacts/tokens`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ token, pubkey }),
    });
    return await res.text();
  } catch (error) {
    console.log(error);
  }
}

async function updateContact(token, id) {
  try {
    const res = await fetch(`http://${ip}/contacts/1`, {
      method: "PUT",
      headers: { "x-user-token": token, "Content-Type": "application/json" },
      body: JSON.stringify({ alias: "admin_user" }),
    });
    return await res.json();
  } catch (error) {
    console.log(error);
  }
}

await testSignup();

async function getToken() {
  const loginRes = await login();
  if (!loginRes.token) return console.error("failed");
  const txt = JSON.stringify({
    type: "GetToken",
  });
  const res = await fetch(`http://${swarm_ip}/api/cmd?txt=${txt}&tag=SWARM`, {
    method: "GET",
    headers: { "x-jwt": loginRes.token, "Content-Type": "application/json" },
  });
  const j = await res.json();
  console.log(j);
}

async function login() {
  const r = await fetch(`http://${swarm_ip}/api/login`, {
    method: "POST",
    body: JSON.stringify({
      username: "admin",
      password: "password",
    }),
  });

  const result = await r.json();
  return result;
}
