import fetch from "node-fetch";

const ip = "https://relay.swarm14.sphinx.chat";

async function testHas() {
  try {
    let hasadmin = false;
    while (!hasadmin) {
      const doeshaveadmin = await checkAdmin();
      if (doeshaveadmin) hasadmin = true;
    }
    const pubkey = await initial_admin_pubkey();
    console.log("initial_admin_pubkey", pubkey);
  } catch (error) {
    console.log("ERROR:", error);
  }
}

async function checkAdmin() {
  const res = await fetch(`${ip}/has_admin`, {
    method: "GET",
    headers: { "Content-Type": "application/json" },
  });
  const j = await res.json();
  console.log("GET /has_admin:", j);
  return j.response || false;
}

async function initial_admin_pubkey() {
  const res = await fetch(`${ip}/initial_admin_pubkey`, {
    method: "GET",
    headers: { "Content-Type": "application/json" },
  });
  const j = await res.json();
  if (!j.success) {
    throw j.error;
  }
  console.log("GET /initial_admin_pubkey", j);
  return j.response.pubkey;
}

await testHas();
