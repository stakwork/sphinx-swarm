const IS_DEV = window.location.host === "localhost:8080";

let root = "/tribes";

if (IS_DEV) {
  root = "http://localhost:13000/tribes";
}

export async function get_tribes(uuid: String = "") {
  let r;
  if (!uuid) {
    r = await fetch(`${root}`);
  } else {
    r = await fetch(`${root}/${uuid}`);
  }
  const result = await r.json();
  return result;
}
