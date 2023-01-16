import { writable } from "svelte/store";

export function localStorageStore<T>(storageKey: string, initialValue: T) {
  let d = {};
  try {
    d = JSON.parse(localStorage.getItem(storageKey));
  } catch (e) {}
  const init = d || initialValue;
  const { subscribe, update, set } = writable<T>(init as T);
  subscribe((state) => {
    if (!state) return;
    debounce(
      storageKey,
      () => {
        localStorage.setItem(storageKey, JSON.stringify(state));
      },
      420
    );
  });
  return {
    subscribe,
    update,
    set,
  };
}

const inDebounce = {};
function debounce(key, func, delay) {
  const context = this;
  const args = arguments;
  if (inDebounce[key]) {
    clearTimeout(inDebounce[key]);
  }
  inDebounce[key] = setTimeout(() => func.apply(context, args), delay);
}
