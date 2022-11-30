const IS_DEV = window.location.host === "localhost:8080";


const formatUrl = (url: string): string => {
  if (url.includes("http://" || "https://")) {
    return url;
  }

  return IS_DEV ? "http://localhost:13000" : `https://${url}` ;
};

export async function get_tribes(url: string, uuid: string = "") {
  let r;

  if (!uuid) {
    r = await fetch(`${formatUrl(url)}/tribes`);
  } else {
    r = await fetch(`${formatUrl(url)}/tribes/${uuid}`);
  }

  const result = await r.json();
  return result;
}

export async function get_people(url: string) {
  const r = await fetch(`${formatUrl(url)}/people`);
  const result = await r.json();
  return result;
}
