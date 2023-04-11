import { writable } from "svelte/store";
import { userKey, type TokenData } from "../../../../../app/src/api/cmd";
import * as api from "../../../../../app/src/api";
import { decode } from "js-base64";

export interface Remote {
  root: string;
  admin: string;
}

const initialRemotes = [
  {
    root: "swarm4.sphinx.chat",
    admin: "kevkevin",
  },
  {
    root: "swarm5.sphinx.chat",
    admin: "Paul",
  },
  {
    root: "swarm7.sphinx.chat",
    admin: "Sam",
  },
];

export const remotes = writable<Remote[]>(initialRemotes);

export const activeUser = writable<string>("_");

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

export const logoutUser = () => {
  localStorage.setItem(userKey, "");
  return activeUser.set("");
};

/*
 * Call to get user token from localstorage
 * and save to store
 */
// saveUserToStore();
