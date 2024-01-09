export const IS_DEV =
  window.location.host === "localhost:5173" ||
  window.location.host === "127.0.0.1:5173";

export let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
  // root = "https://app.swarm9.sphinx.chat/api";
}

const mode = import.meta.env.MODE;

if (mode === "super") {
  root = "https://app.superadmin.sphinx.chat/api";
}

type CmdType =
  | "Swarm"
  | "Relay"
  | "Bitcoind"
  | "Lnd"
  | "Cln"
  | "Proxy"
  | "Hsmd";

export type Cmd =
  | "GetConfig"
  | "ListUsers"
  | "AddUser"
  | "GetInfo"
  | "GetContainerLogs"
  | "TestMine"
  | "ListChannels"
  | "AddPeer"
  | "ListPeers"
  | "AddChannel"
  | "GetBalance"
  | "NewAddress"
  | "ListWallets"
  | "ListVersions"
  | "UpdateInstance"
  | "AddInvoice"
  | "PayInvoice"
  | "PayKeysend"
  | "AddDefaultTribe"
  | "RemoveDefaultTribe"
  | "GetChats"
  | "GetToken"
  | "ListContainers"
  | "StartContainer"
  | "StopContainer"
  | "UpdateNode"
  | "ListFunds"
  | "CloseChannel"
  | "ListInvoices"
  | "ListPays"
  | "ListPayments"
  | "GetStatistics"
  | "ListPendingChannels"
  | "GetClients"
  | "AddBoltwallAdminPubkey"
  | "GetBoltwallSuperAdmin"
  | "AddBoltwallSubAdminPubkey"
  | "ListAdmins"
  | "DeleteSubAdmin"
  | "ListPaidEndpoint"
  | "UpdatePaidEndpoint";

interface CmdData {
  cmd: Cmd;
  content?: any;
}

export interface TokenData {
  exp: number;
  user: number;
}

export const userKey = "SPHINX_TOKEN";

export async function send_cmd(type: CmdType, data: CmdData, tag?: string) {
  const txt = JSON.stringify({ type, data });
  let ret = "";
  try {
    const r = await fetch(`${root}/cmd?txt=${txt}&tag=${tag || "SWARM"}`, {
      headers: {
        "x-jwt": localStorage.getItem(userKey),
      },
    });
    ret = await r.text();
    const jj = JSON.parse(ret);
    if (jj && jj["stack_error"]) {
      console.warn("=> cmd err:", jj["stack_error"]);
      return jj["stack_error"];
    }
    return jj;
  } catch (e) {
    console.warn("=> cmd error:", ret, e);
  }
}
