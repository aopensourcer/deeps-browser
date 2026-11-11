import type { Reply, State, Tab } from "./types";

declare global {
  interface Window {
    ipc?: { postMessage(message: string): void };
    __deeps?: {
      resolve(id: number, reply: Reply): void;
      setState(state: State): void;
      focusAddress(): void;
    };
  }
}

type Waiter = { resolve: (reply: Reply) => void; timer: number };

const waiters = new Map<number, Waiter>();
const listeners = new Set<(state: State) => void>();
let focusHandler: (() => void) | null = null;
let seq = 1;
let installed = false;

function bridge() {
  return typeof window === "undefined" ? null : (window.ipc ?? null);
}

export function call<T = unknown>(
  cmd: string,
  args: Record<string, unknown> = {},
  timeout = 4000,
): Promise<Reply<T>> {
  const ipc = bridge();
  if (!ipc) return Promise.resolve(mock(cmd, args) as Reply<T>);

  const id = seq++;
  return new Promise<Reply<T>>((resolve) => {
    const timer = window.setTimeout(() => {
      waiters.delete(id);
      resolve({ ok: false, error: "timeout" });
    }, timeout);

    waiters.set(id, { resolve: resolve as (reply: Reply) => void, timer });

    try {
      ipc.postMessage(JSON.stringify({ id, cmd, args }));
    } catch (error) {
      window.clearTimeout(timer);
      waiters.delete(id);
      resolve({ ok: false, error: String(error) });
    }
  });
}

export function onState(listener: (state: State) => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function onFocusAddress(listener: () => void) {
  focusHandler = listener;
}

export function install() {
  if (installed) return;
  installed = true;

  window.__deeps = {
    resolve(id, reply) {
      const waiter = waiters.get(id);
      if (!waiter) return;
      window.clearTimeout(waiter.timer);
      waiters.delete(id);
      waiter.resolve(reply);
    },
    setState(state) {
      listeners.forEach((listener) => listener(state));
    },
    focusAddress() {
      focusHandler?.();
    },
  };
}

/* ------------------------------------------------------------------ */
/* Mock bridge: lets the chrome run in a normal browser via `next dev`  */
/* ------------------------------------------------------------------ */

let demo: State | null = null;

function demoTab(id: number, title: string, url: string, loading = false): Tab {
  return { id, title, url, loading };
}

function demoState(): State {
  return {
    tabs: [
      demoTab(1, "Rust (programming language) - Wikipedia", "https://en.wikipedia.org/wiki/Rust_(programming_language)"),
      demoTab(2, "GitHub", "https://github.com/"),
      demoTab(3, "Nuova tab", "http://127.0.0.1:41234/"),
    ],
    active: 2,
    bookmarked: false,
    maximized: false,
    overlay: false,
    sidebarCollapsed: false,
    downloadsOpen: false,
    downloadPrompt: [],
    dialogs: [],
    photoUrl: "",
    downloads: [
      {
        id: 9001,
        url: "https://example.com/deeps-notes.zip",
        name: "deeps-notes.zip",
        path: "C:\\Users\\Demo\\Downloads\\deeps-notes.zip",
        state: "done",
        started: Date.now() - 120000,
        finished: Date.now() - 90000,
        received: 4820000,
        total: 4820000,
      },
    ],
    origin: "http://127.0.0.1:41234",
    engines: [
      { key: "ddg", label: "DuckDuckGo", initial: "d" },
      { key: "google", label: "Google", initial: "g" },
      { key: "bing", label: "Bing", initial: "b" },
      { key: "startpage", label: "Startpage", initial: "s" },
      { key: "yahoo", label: "Yahoo", initial: "y" },
      { key: "ecosia", label: "Ecosia", initial: "e" },
      { key: "qwant", label: "Qwant", initial: "q" },
      { key: "wikipedia", label: "Wikipedia", initial: "w" },
    ],
    sites: [],
    settings: {
      engine: "ddg",
      engineLabel: "DuckDuckGo",
      shortcuts: true,
      adblock: true,
      tabLayout: "top",
      tabShape: "rounded",
      accent: "#6366f1",
      wallpaper: "none",
      saveLogins: true,
      language: "en",
      translateFrom: "auto",
      translateTo: "it",
      onboarded: true,
      downloadDir: "",
      askDownloadDir: true,
      hasPhoto: false,
    },
  };
}

function emit() {
  if (!demo) return;
  const snapshot = demo;
  listeners.forEach((listener) => listener(snapshot));
}

function mock(cmd: string, args: Record<string, unknown>): Reply {
  if (!demo) demo = demoState();

  switch (cmd) {
    case "ready":
      return { ok: true, data: demo };

    case "select_tab": {
      const id = Number(args.id);
      if (demo.tabs.some((tab) => tab.id === id)) demo = { ...demo, active: id, overlay: false };
      emit();
      return { ok: true };
    }

    case "new_tab": {
      const id = Math.max(0, ...demo.tabs.map((tab) => tab.id)) + 1;
      demo = {
        ...demo,
        tabs: [...demo.tabs, demoTab(id, "Nuova tab", String(args.url ?? "http://127.0.0.1:41234/"))],
        active: id,
        bookmarked: false,
        overlay: false,
      };
      emit();
      return { ok: true };
    }

    case "close_tab": {
      const id = Number(args.id ?? demo.active);
      const tabs = demo.tabs.filter((tab) => tab.id !== id);
      if (tabs.length === 0) return { ok: true };
      const index = demo.tabs.findIndex((tab) => tab.id === id);
      demo = { ...demo, tabs, active: tabs[Math.min(index, tabs.length - 1)].id };
      emit();
      return { ok: true };
    }

    case "navigate": {
      const url = String(args.url ?? "");
      demo = {
        ...demo,
        tabs: demo.tabs.map((tab) =>
          tab.id === demo!.active
            ? { ...tab, title: url.replace(/^https?:\/\//, ""), url, loading: false, icon: null }
            : tab,
        ),
      };
      emit();
      return { ok: true, data: { notice: null } };
    }

    case "prev_tab":
    case "next_tab": {
      const index = demo.tabs.findIndex((tab) => tab.id === demo!.active);
      const delta = cmd === "next_tab" ? 1 : -1;
      const next = (index + delta + demo.tabs.length) % demo.tabs.length;
      demo = { ...demo, active: demo.tabs[next].id };
      emit();
      return { ok: true };
    }

    case "overlay": {
      const open = Boolean(args.open);
      demo = { ...demo, overlay: open };
      emit();
      return { ok: true };
    }

    case "sidebar": {
      demo = { ...demo, sidebarCollapsed: Boolean(args.collapsed) };
      emit();
      return { ok: true };
    }

    case "import":
      return { ok: true, data: { added: 0, skipped: 0, sources: [] } };

    case "default_browser":
      return { ok: true, data: { registered: true, path: "" } };

    case "translate_page":
      return { ok: true };

    case "open": {
      const page = String(args.page ?? "");
      const url = `http://127.0.0.1:41234/${page}`;
      demo = {
        ...demo,
        overlay: false,
        tabs: demo.tabs.map((tab) =>
          tab.id === demo!.active
            ? { ...tab, title: page ? page[0].toUpperCase() + page.slice(1) : "Nuova tab", url }
            : tab,
        ),
      };
      emit();
      return { ok: true };
    }

    case "window_max": {
      demo = { ...demo, maximized: !demo.maximized };
      emit();
      return { ok: true, data: { maximized: demo.maximized } };
    }

    case "bookmark_toggle": {
      demo = { ...demo, bookmarked: !demo.bookmarked };
      emit();
      return { ok: true, data: { bookmarked: demo.bookmarked } };
    }

    case "set_shortcuts": {
      demo = {
        ...demo,
        settings: { ...demo.settings, shortcuts: Boolean(args.enabled) },
      };
      emit();
      return { ok: true, data: { notice: null } };
    }

    case "set_adblock": {
      demo = {
        ...demo,
        settings: { ...demo.settings, adblock: Boolean(args.enabled) },
      };
      emit();
      return { ok: true, data: { notice: "Riavvia Deeps per applicare Adblock" } };
    }

    case "save_settings": {
      const current = demo;
      const patch = args as Partial<State["settings"]>;
      const label = current.engines.find(
        (engine) => engine.key === String(args.engine ?? current.settings.engine),
      )?.label;
      demo = {
        ...current,
        settings: {
          ...current.settings,
          ...patch,
          engineLabel: label ?? current.settings.engineLabel,
        },
      };
      emit();
      return { ok: true, data: { notice: "Impostazioni salvate" } };
    }

    case "move_tab": {
      const id = Number(args.id);
      const before = Number(args.before);
      const after = Boolean(args.after);
      const from = demo.tabs.findIndex((tab) => tab.id === id);
      const target = demo.tabs.findIndex((tab) => tab.id === before);
      if (from < 0 || target < 0) return { ok: true };
      let to = after ? target + 1 : target;
      const tabs = [...demo.tabs];
      const [tab] = tabs.splice(from, 1);
      if (from < to) to -= 1;
      tabs.splice(Math.min(to, tabs.length), 0, tab);
      demo = { ...demo, tabs };
      emit();
      return { ok: true };
    }

    default:
      return { ok: true };
  }
}
