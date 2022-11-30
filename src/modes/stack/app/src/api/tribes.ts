const IS_DEV = window.location.host === "localhost:13000";

let root = "/";

if (IS_DEV) {
  root = "http://localhost:13000";
}

export async function get_tribes(uuid: String = "") {
  let r;

  if (!uuid) {
    r = await fetch(`${root}/tribe`);
  } else {
    r = await fetch(`${root}/tribe/${uuid}`);
  }

  const result = await r.json();
  return result;
}

export async function get_people() {
  const r = await fetch(`${root}/people`);
  const result = await r.json();
  return result;
}
