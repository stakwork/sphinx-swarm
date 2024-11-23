export const IS_DEV =
  window.location.host === "localhost:5173" ||
  window.location.host === "127.0.0.1:5173";

export let root = "/api";
if (IS_DEV) {
  root = "http://localhost:8000/api";
  // root = "https://app.v2.sphinx.chat/api";
}

const mode = import.meta.env.MODE;

if (mode === "super") {
  root = "https://app.superadmin.sphinx.chat/api";
  // root = "http://localhost:8005/api";
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
  | "ListPeerChannels"
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
  | "AddBoltwallUser"
  | "ListAdmins"
  | "DeleteSubAdmin"
  | "ListPaidEndpoint"
  | "UpdatePaidEndpoint"
  | "UpdateSwarm"
  | "UpdateBoltwallAccessibility"
  | "GetBoltwallAccessibility"
  | "GetSecondBrainAboutDetails"
  | "GetFeatureFlags"
  | "UpdateSecondBrainAbout"
  | "UpdateFeatureFlags"
  | "GetImageDigest"
  | "GetDockerImageTags"
  | "UpdateUser"
  | "GetApiToken"
  | "AddNewSwarm"
  | "UpdateSwarm"
  | "DeleteSwarm"
  | "GetChildSwarmConfig"
  | "GetChildSwarmContainers"
  | "StopChildSwarmContainers"
  | "StartChildSwarmContainers"
  | "UpdateChildSwarmContainers"
  | "CreateNewEc2Instance"
  | "GetAwsInstanceTypes"
  | "RestartContainer"
  | "RestartChildSwarmContainers"
  | "GetSignedInUserDetails"
  | "UpdateAwsInstanceType"
  | "GetInstanceType"
  | "GetAllImageActualVersion"
  | "GetSwarmChildImageVersions"
  | "ChangeChildSwarmPassword"
  | "GetLightningBotsDetails"
  | "ChangeLigthningBotLabel";

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
  const encodedTxt = encodeURIComponent(txt);
  let ret = "";
  try {
    const r = await fetch(
      `${root}/cmd?txt=${encodedTxt}&tag=${tag || "SWARM"}`,
      {
        headers: {
          "x-jwt": localStorage.getItem(userKey),
        },
      }
    );

    ret = await r.text();
    try {
      const jj = JSON.parse(ret);
      if (jj && jj["stack_error"]) {
        console.warn("=> cmd err:", jj["stack_error"]);
        return jj["stack_error"];
      }
      return jj;
    } catch (error) {
      console.log("error parsing json: ", error);
      return ret;
    }
  } catch (e) {
    console.warn("=> cmd error:", ret, e);
    console.log(e);
  }
}
