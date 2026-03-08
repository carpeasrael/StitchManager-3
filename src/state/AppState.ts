import type { State } from "../types/index";

type Listener<K extends keyof State> = (value: State[K]) => void;

const initialState: State = {
  folders: [],
  selectedFolderId: null,
  files: [],
  selectedFileId: null,
  searchQuery: "",
  formatFilter: null,
  settings: {},
  theme: "hell",
};

class AppStateClass {
  private state: State = { ...initialState };
  private listeners = new Map<keyof State, Set<Listener<keyof State>>>();

  get<K extends keyof State>(key: K): State[K] {
    const value = this.state[key];
    if (Array.isArray(value)) {
      return [...value] as State[K];
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
