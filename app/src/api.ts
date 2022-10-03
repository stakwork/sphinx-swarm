import { writable, get } from "svelte/store";

export const rez = writable<string[]>([]);

export const logs = writable<string[]>([]);

export const nodes = writable<Object>({});

export const tag = writable<string>("");

export const connected = writable<boolean>(false);

const IS_DEV = window.location.host === "localhost:8080";
const DEV_TAG = "2kuKw8";

let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
}

export async function send_cmd(txt: string) {
  let ctag = get(tag);
  if (!ctag) return console.error("not logged in");
  const r = await fetch(`${root}/cmd?txt=${txt}&tag=${ctag}`);
  const newtxt = await r.text();
  const txts = newtxt.split("\n").filter((a) => a);
  if (txts.length) {
    rez.update((r) => [...txts, ...r]);
  }
  return newtxt;
}

fetch(`nodes.json`)
  .then((r) => r.json())
  .then((data) => {
    nodes.set(data);
    if (IS_DEV) login(DEV_TAG);
  });

export async function login(n): Promise<boolean> {
  const current_nodes = get(nodes);
  if (current_nodes[n]) {
    tag.set(n);
    await get_logs(n);
    logstream(n);
    return true;
  }
  return false;
}

export async function get_logs(tag) {
  const r = await fetch(`${root}/logs?tag=${tag}`);
  const lg = await r.json();
  if (Array.isArray(lg)) {
    logs.set(lg.reverse());
  }
  return lg;
}

export function logstream(tag) {
  subscribe(`${root}/logstream?tag=${tag}`);
}

function subscribe(uri: string) {
  var retryTime = 1;

  function connect(uri) {
    const events = new EventSource(uri);

    events.addEventListener("message", (ev) => {
      try {
        console.log(ev.data);
        logs.update((r) => [...r, ev.data]);
      } catch (e) {
        console.log("could parse incoming msg", e);
      }
    });

    events.addEventListener("open", () => {
      connected.set(true);
    });

    events.addEventListener("error", () => {
      connected.set(false);
      events.close();
      let timeout = retryTime;
      retryTime = Math.min(64, retryTime * 2);
      console.log(`connection lost. attempting to reconnect in ${timeout}s`);
      setTimeout(() => connect(uri), (() => timeout * 1000)());
    });
  }

  connect(uri);
}
