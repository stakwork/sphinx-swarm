import { writable } from "svelte/store";
import { userKey, type TokenData } from "../../../../../app/src/api/cmd";
import * as api from "../../../../../app/src/api";
import { decode } from "js-base64";
import type {
  Tribe,
  Remote,
  ILightningBot,
  ReservedRemote,
} from "./types/types";

export const remotes = writable<Remote[]>([]);

export const reservedRemotes = writable<ReservedRemote[]>([]);

export const activeUser = writable<string>();

export const selectedNode = writable<string>();

export const tribes = writable<{ [k: string]: Tribe[] }>({});

export const lightningBots = writable<ILightningBot[]>([]);

export const anthropicKeys = writable<string[]>([]);

export const saveUserToStore = async (user: string = "") => {
  if (user) {
    localStorage.setItem(userKey, user);
    return activeUser.set(user);
  }

  let storageUser = localStorage.getItem(userKey);

  if (storageUser) {
    const jwts = storageUser.split(".");

    const decodedData: TokenData = JSON.parse(decode(jwts[1]));

    if (decodedData.exp * 1000 > Date.now()) {
      const refresh = await api.swarm.refresh_token(storageUser);

      // save the new token to localstorage
      localStorage.setItem(userKey, refresh.token);
      return activeUser.set(refresh.token);
    }
  }
};

saveUserToStore();

export const logoutUser = () => {
  localStorage.setItem(userKey, "");
  return activeUser.set("");
};

/*
 * Call to get user token from localstorage
 * and save to store
 */
// saveUserToStore();
