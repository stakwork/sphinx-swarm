import fetch from "node-fetch";

async function testSignup() {
  const qrToken = `claim::localhost:3000::MDZ2Tm9zVlBBNTND`;
  const arr = qrToken.split("::");
  const ip = arr[1];
  const token = decodeToken(arr[2]);
  const contacts = await getContact(token, ip);
  const contact = contacts.response.contacts[0];
  const updatedContact = await updateContact(ip, token, contact.id);
  if (updatedContact.success) {
    console.log("====> admin setup successfully <======");
  } else {
    console.log("=====> An error occured <======");
  }
}

function decodeToken(token) {
  return Buffer.from(token, "base64").toString("utf8");
}

async function getContact(token, ip) {
  try {
    const res = await fetch(`http://${ip}/contacts`, {
      method: "GET",
      headers: { "x-user-token": token, "Content-Type": "application/json" },
    });

    return await res.json();
  } catch (error) {
    console.log(error);
  }
}

async function updateContact(ip, token, id) {
  try {
    const res = await fetch(`http://${ip}/contacts/${id}`, {
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
