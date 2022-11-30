const IS_DEV = window.location.host === "localhost:8080";

let root = "https://tribes.sphinx.chat";

if (IS_DEV) {
  root = "http://localhost:13000";
}

export async function get_tribes(uuid: String = "") {
  let r;

  if (!uuid) {
    r = await fetch(`${root}/tribes`);
  } else {
    r = await fetch(`${root}/tribes/${uuid}`);
  }

  const result = await r.json();
  return result;
}

export async function get_people() {
  const r = await fetch(`${root}/people`);
  const result = await r.json();
  return result;
}
