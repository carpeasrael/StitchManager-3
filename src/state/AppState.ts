import type { State } from "../types/index";

type Listener<K extends keyof State> = (value: State[K]) => void;

const initialState: State = {
  folders: [],
  selectedFolderId: null,
  files: [],
  selectedFileId: null,
  selectedFileIds: [],
  searchQuery: "",
  searchParams: {},
  formatFilter: null,
  settings: {},
  theme: "hell",
  toasts: [],
  usbDevices: [],
  expandedFolderIds: [],
};

class AppStateClass {
  private state: State = { ...initialState };
  private listeners = new Map<keyof State, Set<Listener<keyof State>>>();

  /** Returns a direct reference to the state value (no copy).
   *  The `Readonly` type is compile-time only — do NOT mutate the
   *  returned value at runtime. Use `set()` or `update()` to change state. */
  getRef<K extends keyof State>(key: K): Readonly<State[K]> {
    return this.state[key];
  }

  get<K extends keyof State>(key: K): State[K] {
    const value = this.state[key];
    if (Array.isArray(value)) {
      return value.map((item) =>
        item !== null && typeof item === "object" ? { ...item } : item
      ) as State[K];
    }
    if (value !== null && typeof value === "object") {
      return { ...value } as State[K];
    }
    return value;
  }

  set<K extends keyof State>(key: K, value: State[K]): void {
    this.state[key] = value;
    const set = this.listeners.get(key);
    if (set) {
      set.forEach((listener) => (listener as Listener<K>)(value));
    }
  }

  /** Atomically read-modify-write a state key. The updater receives the
   *  current value and must return the new value. Listeners fire once
   *  with the result. */
  update<K extends keyof State>(key: K, updater: (current: State[K]) => State[K]): void {
    this.set(key, updater(this.get(key)));
  }

  on<K extends keyof State>(
    key: K,
    listener: Listener<K>
  ): () => void {
    if (!this.listeners.has(key)) {
      this.listeners.set(key, new Set());
    }
    this.listeners.get(key)!.add(listener as Listener<keyof State>);

    return () => {
      const set = this.listeners.get(key);
      if (set) {
        set.delete(listener as Listener<keyof State>);
      }
    };
  }
}

export const appState = new AppStateClass();
