import { writable } from "svelte/store";
import { localStorageStore } from "./storage";
import type { Node } from "./nodes";

export const selectedNode = writable<Node>();
