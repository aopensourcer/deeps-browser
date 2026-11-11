"use client";

import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type MouseEvent as ReactMouseEvent,
} from "react";
import { Icon, type IconName } from "@/components/Icon";
import { Vortex } from "@/components/Vortex";
import { call, install, onFocusAddress, onState } from "@/lib/ipc";
import { t, type Key, type Lang } from "@/lib/i18n";
import type {
  ImportReport,
  State,
  Tab,
  TabLayout,
  TabShape,
  Wallpaper,
} from "@/lib/types";

const EMPTY: State = {
  tabs: [],
  active: 0,
  bookmarked: false,
  maximized: false,
  overlay: false,
  sidebarCollapsed: false,
  origin: "",
  downloads: [],
  downloadsOpen: false,
  downloadPrompt: [],
  dialogs: [],
  photoUrl: "",
  engines: [],
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

const ACCENTS = ["#6366f1", "#0ea5e9", "#22c55e", "#f59e0b", "#ef4444", "#ec4899", "#8b5cf6", "#14b8a6"];
const WALLPAPERS: { key: Wallpaper; label: Key }[] = [
  { key: "none", label: "wallNone" },
  { key: "aurora", label: "wallAurora" },
  { key: "mesh", label: "wallMesh" },
  { key: "grid", label: "wallGrid" },
  { key: "dunes", label: "wallDunes" },
  { key: "stars", label: "wallStars" },
];

function loopbackPath(url: string, origin: string): string | null {
  if (!origin || !url.startsWith(origin)) return null;
  const rest = url.slice(origin.length);
  if (rest !== "" && !rest.startsWith("/")) return null;
  return rest.replace(/^\/+/, "");
}

function isInternal(url: string, origin: string) {
  return loopbackPath(url, origin) !== null;
}

function pretty(url: string, origin: string) {
  if (!url) return "";
  const path = loopbackPath(url, origin);
  if (path !== null) return path ? `deeps://${path}` : "";
  return url.replace(/^https?:\/\//, "").replace(/\/$/, "");
}

function hostOf(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}

/** Title of a page's script dialog: "Deeps" for internal pages, the bare
 *  domain (no port) for everything else. */
function sourceOf(url: string, internalOrigin: string) {
  if (internalOrigin && url.startsWith(internalOrigin)) return "Deeps";
  try {
    return new URL(url).hostname.replace(/^www\./, "") || "Deeps";
  } catch {
    return "Deeps";
  }
}

function siteLabel(url: string) {
  try {
    const host = new URL(url).hostname.replace(/^www\./, "");
    const parts = host.split(".");
    if (parts.length >= 2) {
      const name = parts[parts.length - 2];
      if (name.length >= 3) return name;
    }
    return host;
  } catch {
    return "";
  }
}

function hueOf(seed: string) {
  let hue = 0;
  for (let i = 0; i < seed.length; i += 1) hue = (hue * 31 + seed.charCodeAt(i)) % 360;
  return hue;
}

function initialOf(tab: Tab) {
  return (siteLabel(tab.url) || tab.title || "?").charAt(0).toUpperCase();
}

function labelOf(tab: Tab, lang: Lang = "en") {
  return tab.title || hostOf(tab.url) || (lang === "it" ? "Nuova tab" : "New tab");
}

export default function Chrome() {
  const [state, setState] = useState<State>(EMPTY);
  const [value, setValue] = useState("");
  const [editing, setEditing] = useState(false);
  const [panel, setPanel] = useState<"menu" | "extensions">("menu");
  const [toast, setToast] = useState("");
  const [stripWidth, setStripWidth] = useState(1280);
  const [dragging, setDragging] = useState<number | null>(null);
  const [broken, setBroken] = useState<Set<string>>(new Set());
  const [autoHide, setAutoHide] = useState(false);
  const [saveFolder, setSaveFolder] = useState("");
  const [alwaysHere, setAlwaysHere] = useState(false);
  const [dialogText, setDialogText] = useState("");

  const input = useRef<HTMLInputElement | null>(null);
  const stripRef = useRef<HTMLDivElement | null>(null);
  const barRef = useRef<HTMLDivElement | null>(null);
  const toastTimer = useRef(0);
  const enterTimer = useRef(0);
  const leaveTimer = useRef(0);
  const autoHideRef = useRef(false);
  const verticalRef = useRef(false);
  const downloadSeen = useRef<Record<number, string> | null>(null);

  const lang: Lang = state.settings.language === "it" ? "it" : "en";
  const tr = (key: Key, vars?: Record<string, string | number>) => t(lang, key, vars);

  const vertical = state.settings.tabLayout === "vertical";
  const collapsed = vertical && state.sidebarCollapsed;
  autoHideRef.current = autoHide;
  verticalRef.current = vertical;
  const onboarding = !state.settings.onboarded;
  const active = state.tabs.find((tab) => tab.id === state.active) ?? state.tabs[0];
  const activeUrl = active?.url ?? "";
  const count = Math.max(1, state.tabs.length);
  const available = Math.max(140, stripWidth - 224);
  const tabWidth = Math.max(78, Math.min(206, Math.floor((available - (count - 1) * 2) / count)));
  const running = state.downloads.filter((entry) => entry.state === "running").length;
  const prompt = state.downloadPrompt[0];
  const dialog = state.dialogs[0];
  const latestRunning = [...state.downloads].reverse().find((entry) => entry.state === "running");
  const progress =
    latestRunning && latestRunning.total > 0
      ? Math.min(1, latestRunning.received / latestRunning.total)
      : 0;

  function notify(message: string) {
    setToast(message);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(""), 2800);
  }

  useEffect(() => {
    install();
    const off = onState(setState);
    onFocusAddress(() => {
      input.current?.focus();
      input.current?.select();
    });
    void call<State | null>("ready").then((reply) => {
      if (reply.ok && reply.data) setState(reply.data);
    });
    return () => {
      off();
      window.clearTimeout(toastTimer.current);
      window.clearTimeout(enterTimer.current);
      window.clearTimeout(leaveTimer.current);
    };
  }, []);

  useEffect(() => {
    if (!editing) setValue(pretty(activeUrl, state.origin));
  }, [activeUrl, editing]);

  useEffect(() => {
    setEditing(false);
  }, [state.active]);

  useEffect(() => {
    const tab = stripRef.current?.querySelector<HTMLElement>(".tab.active");
    tab?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [state.active, state.tabs.length]);

  useEffect(() => {
    const current: Record<number, string> = {};
    for (const entry of state.downloads) current[entry.id] = entry.state;
    const previous = downloadSeen.current;
    downloadSeen.current = current;
    if (!previous) return;
    for (const entry of state.downloads) {
      const before = previous[entry.id];
      if (!before) {
        if (entry.state === "running") notify(`${tr("downloadStarted")}: ${entry.name}`);
        continue;
      }
      if (before !== entry.state && entry.state === "done") {
        notify(`${tr("downloadDone")}: ${entry.name}`);
      } else if (before !== entry.state && entry.state === "failed") {
        notify(`${tr("downloadFailed")}: ${entry.name}`);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.downloads]);

  useEffect(() => {
    const element = barRef.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (width) setStripWidth(width);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Keep the folder field in sync with the download waiting for a destination.
  useEffect(() => {
    if (prompt) {
      setSaveFolder(prompt.folder);
      setAlwaysHere(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [prompt?.id]);

  // A new page dialog starts from its default value.
  useEffect(() => {
    if (dialog) setDialogText(dialog.defaultText || "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dialog?.id]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const report = () => void call("theme", { dark: media.matches });
    report();
    media.addEventListener("change", report);
    return () => media.removeEventListener("change", report);
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      const mod = event.ctrlKey || event.metaKey;

      if (event.key === "Escape") {
        if (dialog) {
          event.preventDefault();
          void call("dialog_cancel", { id: dialog.id });
          return;
        }
        if (prompt) {
          event.preventDefault();
          void call("download_cancel", { id: prompt.id });
          return;
        }
        if (state.downloadsOpen) {
          event.preventDefault();
          void call("downloads_panel", { open: false });
          return;
        }
        if (state.overlay) {
          event.preventDefault();
          void call("overlay", { open: false });
        }
        return;
      }

      if (!state.settings.shortcuts || onboarding) return;

      // A page dialog is modal: only Escape and Enter work while it is open.
      if (dialog) return;

      if (event.altKey && key === "arrowleft") {
        event.preventDefault();
        void call("back");
        return;
      }
      if (event.altKey && key === "arrowright") {
        event.preventDefault();
        void call("forward");
        return;
      }

      if (!mod || event.altKey) return;

      if (key === "l") {
        event.preventDefault();
        input.current?.focus();
        input.current?.select();
        return;
      }
      if (key === "k") {
        event.preventDefault();
        setPanel("menu");
        void call("overlay", { open: !state.overlay });
        return;
      }
      if (key === "t") {
        event.preventDefault();
        void call("new_tab");
        return;
      }
      if (key === "w") {
        event.preventDefault();
        void call("close_tab");
        return;
      }
      if (key === "r") {
        event.preventDefault();
        void call("reload");
        return;
      }
      if (key === "i" && event.shiftKey) {
        event.preventDefault();
        void call("devtools");
        return;
      }
      if (key === "tab") {
        event.preventDefault();
        void call(event.shiftKey ? "prev_tab" : "next_tab");
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [state.settings.shortcuts, state.overlay, state.downloadsOpen, prompt?.id, dialog?.id, onboarding]);

  useEffect(() => {
    let zone = "";
    const edge = 5;
    const compute = (x: number, y: number) => {
      const width = window.innerWidth;
      const height = window.innerHeight;
      const left = x < edge && !vertical;
      const right = x > width - edge;
      const top = y < edge;
      const bottom = state.overlay && y > height - edge;
      if (top && left) return "northwest";
      if (top && right) return "northeast";
      if (bottom && left) return "southwest";
      if (bottom && right) return "southeast";
      if (top) return "north";
      if (bottom) return "south";
      if (left) return "west";
      if (right) return "east";
      return "";
    };
    const onMove = (event: PointerEvent) => {
      if (event.buttons !== 0) return;
      const next = compute(event.clientX, event.clientY);
      if (next === zone) return;
      zone = next;
      if (next) document.body.dataset.resize = next;
      else delete document.body.dataset.resize;
    };
    const onDown = (event: PointerEvent) => {
      if (event.button !== 0) return;
      const next = compute(event.clientX, event.clientY);
      if (!next) return;
      event.preventDefault();
      event.stopPropagation();
      void call("resize", { dir: next });
    };
    const onLeave = () => {
      zone = "";
      delete document.body.dataset.resize;
    };
    window.addEventListener("pointermove", onMove, true);
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("pointerleave", onLeave, true);
    return () => {
      window.removeEventListener("pointermove", onMove, true);
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("pointerleave", onLeave, true);
    };
  }, [state.overlay, vertical]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const target = value.trim();
    if (!target) {
      input.current?.focus();
      return;
    }
    setEditing(false);
    input.current?.blur();
    const reply = await call<{ notice: string | null }>("navigate", { url: target });
    if (!reply.ok && reply.error) notify(reply.error);
    else if (reply.data?.notice) notify(reply.data.notice);
  }

  function toggleOverlay(open: boolean) {
    if (open) setPanel("menu");
    void call("overlay", { open });
  }

  function openPage(page: string) {
    setPanel("menu");
    void call("open", { page });
  }

  function toggleDownloads() {
    void call("downloads_panel", { open: !state.downloadsOpen });
  }

  function deleteDownload(id: number) {
    void call("delete_download", { id });
  }

  async function chooseFolder() {
    const reply = await call<{ path: string }>("pick_folder");
    if (!reply.ok) {
      if (reply.error) notify(reply.error);
      return;
    }
    if (reply.data?.path) {
      setSaveFolder(reply.data.path);
      notify(tr("folderChosen"));
    }
  }

  function confirmDownload() {
    if (!prompt) return;
    void call("download_accept", {
      id: prompt.id,
      folder: saveFolder,
      always: alwaysHere,
    });
  }

  function cancelDownload() {
    if (prompt) void call("download_cancel", { id: prompt.id });
  }

  function answerDialog(accept: boolean) {
    if (!dialog) return;
    if (accept) void call("dialog_accept", { id: dialog.id, text: dialogText });
    else void call("dialog_cancel", { id: dialog.id });
  }

  async function toggleBookmark() {
    const reply = await call<{ bookmarked: boolean }>("bookmark_toggle");
    if (!reply.ok) {
      if (reply.error) notify(reply.error);
      return;
    }
    notify(reply.data?.bookmarked ? tr("bookmarkAdded") : tr("bookmarkRemoved"));
  }

  async function toggleShortcuts() {
    const enabled = !state.settings.shortcuts;
    const reply = await call<{ notice: string | null }>("set_shortcuts", { enabled });
    if (!reply.ok) {
      if (reply.error) notify(reply.error);
      return;
    }
    notify(`${tr("shortcuts")} ${enabled ? tr("shortcutsOn") : tr("shortcutsOff")}`);
  }

  async function toggleAdblock() {
    const reply = await call<{ notice: string | null }>("set_adblock", {
      enabled: !state.settings.adblock,
    });
    if (!reply.ok) {
      if (reply.error) notify(reply.error);
      return;
    }
    notify(tr("adblockRestart"));
  }

  function toggleAutoHide() {
    if (autoHide) {
      setAutoHide(false);
      if (state.sidebarCollapsed) void call("sidebar", { collapsed: false });
      notify(tr("sidebarFixed"));
    } else {
      setAutoHide(true);
      notify(tr("sidebarAuto"));
    }
  }

  function sidebarEnter() {
    if (!verticalRef.current || !autoHideRef.current) return;
    window.clearTimeout(leaveTimer.current);
    window.clearTimeout(enterTimer.current);
    enterTimer.current = window.setTimeout(() => {
      void call("sidebar", { collapsed: false });
    }, 150);
  }

  function sidebarLeave() {
    if (!verticalRef.current || !autoHideRef.current) return;
    window.clearTimeout(enterTimer.current);
    window.clearTimeout(leaveTimer.current);
    leaveTimer.current = window.setTimeout(() => {
      void call("sidebar", { collapsed: true });
    }, 240);
  }

  async function importBookmarks() {
    notify(tr("importing"));
    const reply = await call<ImportReport>("import");
    if (!reply.ok) {
      notify(reply.error || tr("importFailed"));
      return;
    }
    const report = reply.data;
    if (!report || (!report.added && !report.skipped)) {
      notify(tr("importNone"));
      return;
    }
    notify(tr("importResult", { added: report.added, skipped: report.skipped }));
  }

  async function makeDefault() {
    const reply = await call<{ registered: boolean; path: string }>("default_browser");
    notify(reply.ok ? tr("defaultOpened") : reply.error || tr("defaultFailed"));
  }

  function stripMouseDown(event: ReactMouseEvent) {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    if (target.closest("button, .tab, .wincontrols")) return;
    void call("drag");
  }

  function stripDoubleClick(event: ReactMouseEvent) {
    const target = event.target as HTMLElement;
    if (target.closest("button, .tab, .wincontrols")) return;
    void call("window_max");
  }

  function markBroken(icon: string) {
    setBroken((previous) => {
      const next = new Set(previous);
      next.add(icon);
      return next;
    });
  }

  const addressIcon = editing ? "search" : activeUrl.startsWith("https://") ? "lock" : "globe";
  const canBookmark = Boolean(activeUrl) && !isInternal(activeUrl, state.origin);

  function faviconFor(tab: Tab) {
    if (tab.loading) return <span className="fav load" aria-hidden="true" />;
    if (isInternal(tab.url, state.origin)) return <Vortex size={14} />;
    if (tab.icon && !broken.has(tab.icon)) {
      return (
        <span className="fav image" aria-hidden="true">
          <img src={tab.icon} alt="" onError={() => markBroken(tab.icon!)} />
        </span>
      );
    }
    return (
      <span className="fav" style={{ "--h": hueOf(siteLabel(tab.url) || labelOf(tab)) } as CSSProperties}>
        {initialOf(tab)}
      </span>
    );
  }

  const appClass = [
    "app",
    vertical ? "vertical" : "",
    collapsed ? "collapsed" : "",
    `shape-${state.settings.tabShape}`,
    `wall-${state.settings.wallpaper}`,
    state.settings.hasPhoto && state.photoUrl ? "has-photo" : "",
    state.downloadsOpen ? "panel-open" : "",
    state.overlay || onboarding || prompt || dialog ? "overlay-open" : "",
    onboarding ? "onboarding" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      className={appClass}
      style={
        {
          "--tabw": `${tabWidth}px`,
          "--accent": state.settings.accent,
          ...(state.settings.hasPhoto && state.photoUrl
            ? { "--photo": `url("${state.photoUrl}")` }
            : {}),
        } as CSSProperties
      }
    >
      <div
        className="strip"
        ref={barRef}
        onMouseDown={stripMouseDown}
        onDoubleClick={stripDoubleClick}
        onMouseEnter={sidebarEnter}
        onMouseLeave={sidebarLeave}
      >
        <button
          className="logo"
          title={state.overlay ? tr("closeMenu") : tr("openMenu")}
          aria-label={tr("menuAria")}
          onClick={() => toggleOverlay(!state.overlay)}
        >
          <Vortex size={17} />
          {state.settings.shortcuts ? <span className="logoDot" aria-hidden="true" /> : null}
        </button>

        {vertical ? (
          <button
            className={`sidecollapse${autoHide ? " on" : ""}`}
            title={autoHide ? tr("pinExpand") : tr("pinCollapse")}
            aria-label={tr("pinAria")}
            aria-pressed={autoHide}
            onClick={toggleAutoHide}
          >
            <Icon name="pin" size={14} />
          </button>
        ) : null}

        {vertical ? (
          <button
            className="newtab side"
            aria-label={tr("newTab")}
            title={`${tr("newTab")} (Ctrl+T)`}
            onClick={() => void call("new_tab")}
          >
            <Icon name="plus" size={15} stroke={1.9} />
            <span>{tr("newTab")}</span>
          </button>
        ) : null}

        <div className="tabs" role="tablist" ref={stripRef}>
          {state.tabs.map((tab) => {
            const selected = tab.id === state.active;
            return (
              <div
                key={tab.id}
                role="tab"
                tabIndex={0}
                draggable
                aria-selected={selected}
                className={`tab${selected ? " active" : ""}${dragging === tab.id ? " dragging" : ""}`}
                title={tab.url || labelOf(tab, lang)}
                onClick={() => void call("select_tab", { id: tab.id })}
                onAuxClick={(event) => {
                  if (event.button === 1) {
                    event.preventDefault();
                    void call("close_tab", { id: tab.id });
                  }
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    void call("select_tab", { id: tab.id });
                  }
                }}
                onDragStart={(event) => {
                  setDragging(tab.id);
                  event.dataTransfer.effectAllowed = "move";
                  event.dataTransfer.setData("text/plain", String(tab.id));
                }}
                onDragEnd={() => setDragging(null)}
                onDragOver={(event) => {
                  event.preventDefault();
                  event.dataTransfer.dropEffect = "move";
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  const source = Number(event.dataTransfer.getData("text/plain") || dragging);
                  if (source && source !== tab.id) {
                    const rect = event.currentTarget.getBoundingClientRect();
                    const after = vertical
                      ? event.clientY > rect.top + rect.height / 2
                      : event.clientX > rect.left + rect.width / 2;
                    void call("move_tab", { id: source, before: tab.id, after });
                  }
                  setDragging(null);
                }}
              >
                {faviconFor(tab)}
                <span className="label">{labelOf(tab, lang)}</span>
                <button
                  type="button"
                  className="close"
                  aria-label={tr("closeTab")}
                  title={tr("closeTab")}
                  onClick={(event) => {
                    event.stopPropagation();
                    void call("close_tab", { id: tab.id });
                  }}
                >
                  <Icon name="close" size={11} stroke={2.1} />
                </button>
              </div>
            );
          })}
        </div>

        {!vertical ? (
          <button
            className="newtab"
            aria-label={tr("newTab")}
            title={`${tr("newTab")} (Ctrl+T)`}
            onClick={() => void call("new_tab")}
          >
            <Icon name="plus" size={15} stroke={1.9} />
          </button>
        ) : null}

        <div className="dragzone" />

        {!vertical ? <WinControls maximized={state.maximized} lang={lang} /> : null}
      </div>

      {vertical ? (
        <div className="vtop" onMouseDown={stripMouseDown} onDoubleClick={stripDoubleClick}>
          <span className="vtitle">{active ? labelOf(active, lang) : "Deeps"}</span>
          <WinControls maximized={state.maximized} lang={lang} />
        </div>
      ) : null}

      <div className="toolbar">
        <button
          className="iconbtn"
          title={`${tr("back")} (Alt+←)`}
          aria-label={tr("back")}
          onClick={() => void call("back")}
        >
          <Icon name="back" />
        </button>
        <button
          className="iconbtn"
          title={`${tr("forward")} (Alt+→)`}
          aria-label={tr("forward")}
          onClick={() => void call("forward")}
        >
          <Icon name="forward" />
        </button>
        <button
          className="iconbtn"
          title={`${tr("reload")} (Ctrl+R)`}
          aria-label={tr("reload")}
          onClick={() => void call("reload")}
        >
          <Icon name="reload" size={16} />
        </button>

        <form
          className="addr"
          onSubmit={submit}
          onMouseDown={(event) => {
            if (event.target !== input.current) {
              event.preventDefault();
              input.current?.focus();
              input.current?.select();
            }
          }}
        >
          <Icon name={addressIcon} size={13} stroke={1.8} />
          <input
            ref={input}
            value={value}
            spellCheck={false}
            autoComplete="off"
            autoCapitalize="off"
            placeholder={tr("searchPlaceholder", { engine: state.settings.engineLabel })}
            onFocus={() => {
              setEditing(true);
              setValue(pretty(activeUrl, state.origin));
              requestAnimationFrame(() => input.current?.select());
            }}
            onChange={(event) => setValue(event.target.value)}
            onBlur={() => setEditing(false)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                input.current?.blur();
              }
            }}
          />
        </form>

        <button
          className="iconbtn"
          title={tr("translatePage")}
          aria-label={tr("translatePage")}
          onClick={() => void call("translate_page")}
        >
          <Icon name="translate" size={16} />
        </button>
        <button
          className={`iconbtn star${state.bookmarked ? " on" : ""}`}
          title={state.bookmarked ? tr("removeBookmark") : tr("addBookmark")}
          aria-label={tr("addBookmark")}
          aria-pressed={state.bookmarked}
          disabled={!canBookmark}
          onClick={() => void toggleBookmark()}
        >
          <Icon name={state.bookmarked ? "starFilled" : "star"} size={16} />
        </button>
        <button
          className={`iconbtn downloads${state.downloadsOpen ? " on" : ""}`}
          title={running ? `${tr("downloads")} · ${running} ${tr("downloading")}` : tr("downloads")}
          aria-label={tr("downloads")}
          aria-expanded={state.downloadsOpen}
          onClick={toggleDownloads}
        >
          <Icon name="download" size={16} />
          {running ? (
            <svg className="ring" viewBox="0 0 24 24" aria-hidden="true">
              <circle className="ringtrack" cx="12" cy="12" r="10" />
              <circle
                className="ringfill"
                cx="12"
                cy="12"
                r="10"
                strokeDasharray={`${(progress * 62.832).toFixed(2)} 62.832`}
              />
            </svg>
          ) : null}
        </button>
        <button
          className="iconbtn"
          title={tr("extensions")}
          aria-label={tr("extensions")}
          onClick={() => {
            setPanel("extensions");
            void call("overlay", { open: true });
          }}
        >
          <Icon name="puzzle" size={16} />
        </button>
      </div>

      {state.downloadsOpen && !onboarding && !state.overlay && !dialog ? (
        <div className="dlpanel" role="region" aria-label={tr("downloads")}>
          <div className="dlhead">
            <span className="dltitle">{tr("downloads")}</span>
          </div>
          <div className="dllist">
            {state.downloads.length === 0 ? (
              <div className="dlempty">{tr("downloadPanelEmpty")}</div>
            ) : (
              [...state.downloads]
                .reverse()
                .slice(0, 5)
                .map((entry) => {
                  const percent =
                    entry.total > 0
                      ? Math.min(100, Math.round((entry.received / entry.total) * 100))
                      : 0;
                  return (
                    <div key={entry.id} className={`dlrow ${entry.state}`}>
                      <span className="dlicon" aria-hidden="true">
                        <Icon name="download" size={13} />
                      </span>
                      <div className="dlbody">
                        <span className="dlname" title={entry.path || entry.name}>
                          {entry.name}
                        </span>
                        <span className="dlmeta">
                          {entry.state === "running"
                            ? entry.total > 0
                              ? `${percent}%`
                              : tr("downloading")
                            : entry.state === "failed"
                              ? tr("downloadFailed")
                              : tr("downloadDone")}
                        </span>
                        {entry.state === "running" ? (
                          <span className="dlbar" aria-hidden="true">
                            <span style={{ width: `${percent}%` }} />
                          </span>
                        ) : null}
                      </div>
                      <button
                        type="button"
                        className="dldelete"
                        title={tr("downloadDelete")}
                        aria-label={tr("downloadDelete")}
                        onClick={() => deleteDownload(entry.id)}
                      >
                        <Icon name="trash" size={13} />
                      </button>
                    </div>
                  );
                })
            )}
          </div>
          <button className="dlall" type="button" onClick={() => openPage("downloads")}>
            <Icon name="download" size={13} />
            <span>{tr("seeAllDownloads")}</span>
            <Icon name="forward" size={12} />
          </button>
        </div>
      ) : null}

      {active?.loading ? <div className="loadbar" /> : null}

      {state.overlay && !onboarding ? (
        <div
          className="overlay"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) void call("overlay", { open: false });
          }}
        >
          <div className="sheet" role="dialog" aria-label={tr("menuAria")}>
            <div className="sheethead">
              <Vortex size={26} />
              <div>
                <div className="sheettitle">Deeps</div>
                <div className="sheetsub">{panel === "menu" ? tr("menuTitle") : tr("extensions")}</div>
              </div>
              <kbd className="esc">Esc</kbd>
            </div>

            {panel === "menu" ? (
              <div className="sheetlist">
                <MenuRow icon="plus" label={tr("newTab")} kbd="Ctrl T" onClick={() => { toggleOverlay(false); void call("new_tab"); }} />
                <MenuRow icon="history" label={tr("history")} onClick={() => openPage("history")} />
                <MenuRow icon="bookmark" label={tr("bookmarks")} onClick={() => openPage("bookmarks")} />
                <MenuRow
                  icon="download"
                  label={tr("downloads")}
                  detail={running ? `${running} ${tr("downloading")}` : tr("downloadsDetail")}
                  onClick={() => openPage("downloads")}
                />
                <MenuRow icon="key" label={tr("passwords")} onClick={() => openPage("passwords")} />
                <MenuRow icon="translate" label={tr("translatePage")} detail={tr("translatePageDetail")} onClick={() => { toggleOverlay(false); void call("translate_page"); }} />
                <MenuRow icon="translate" label={tr("translator")} detail={tr("translatorDetail")} onClick={() => openPage("translate")} />
                <MenuRow icon="vpn" label={tr("vpn")} detail={tr("vpnDetail")} onClick={() => openPage("vpn")} />
                <MenuRow icon="settings" label={tr("settings")} onClick={() => openPage("settings")} />
                <MenuRow icon="bookmark" label={tr("importBookmarks")} onClick={() => { toggleOverlay(false); void importBookmarks(); }} />
                <MenuRow icon="globe" label={tr("setDefault")} detail={tr("defaultDetail")} onClick={() => { toggleOverlay(false); void makeDefault(); }} />
                <MenuRow
                  icon="puzzle"
                  label={tr("extensions")}
                  detail={state.settings.adblock ? tr("adblockActive") : tr("adblockOff")}
                  onClick={() => setPanel("extensions")}
                />
                <div className="sheetsep" />
                <MenuRow icon="power" label={tr("quit")} onClick={() => void call("window_close")} />
              </div>
            ) : (
              <div className="sheetlist">
                <div className="extrow">
                  <div className="extmeta">
                    <div className="extname">
                      {tr("adblockName")} <span className="pill">{tr("integrated")}</span>
                    </div>
                    <div className="extdesc">{tr("adblockDesc")}</div>
                  </div>
                  <button
                    className={`miniSwitch${state.settings.adblock ? " on" : ""}`}
                    role="switch"
                    aria-checked={state.settings.adblock}
                    aria-label={tr("adblockName")}
                    onClick={() => void toggleAdblock()}
                  >
                    <span />
                  </button>
                </div>
                <div className="extrow">
                  <div className="extmeta">
                    <div className="extname">
                      {tr("loginSaving")} <span className="pill">{tr("vault")}</span>
                    </div>
                    <div className="extdesc">{tr("loginDesc")}</div>
                  </div>
                  <button
                    className={`miniSwitch${state.settings.saveLogins ? " on" : ""}`}
                    role="switch"
                    aria-checked={state.settings.saveLogins}
                    aria-label={tr("loginSaving")}
                    onClick={() => void call("save_settings", { saveLogins: !state.settings.saveLogins })}
                  >
                    <span />
                  </button>
                </div>
                <div className="extrow">
                  <div className="extmeta">
                    <div className="extname">
                      {tr("translatorName")} <span className="pill">{tr("integrated")}</span>
                    </div>
                    <div className="extdesc">{tr("translatorDesc")}</div>
                  </div>
                  <button className="miniAction" onClick={() => { toggleOverlay(false); openPage("translate"); }}>
                    {tr("open")}
                  </button>
                </div>
                <div className="extrow">
                  <div className="extmeta">
                    <div className="extname">
                      {tr("vpnName")} <span className="pill">WARP</span>
                    </div>
                    <div className="extdesc">{tr("vpnDesc")}</div>
                  </div>
                  <button className="miniAction" onClick={() => { toggleOverlay(false); openPage("vpn"); }}>
                    {tr("open")}
                  </button>
                </div>
                <div className="extnote">{tr("extensionsNote")}</div>
                <button className="sheetback" onClick={() => setPanel("menu")}>
                  <Icon name="back" size={14} /> {tr("backLabel")}
                </button>
              </div>
            )}

            <div className="sheetfoot">
              <button className="foottoggle" onClick={() => void toggleShortcuts()}>
                <span className={state.settings.shortcuts ? "dot on" : "dot"} />
                {tr("shortcuts")} {state.settings.shortcuts ? tr("shortcutsOn") : tr("shortcutsOff")}
              </button>
              <span className="footmeta">{state.settings.engineLabel}</span>
              <span className="footmeta">{vertical ? tr("vertical") : tr("horizontal")}</span>
            </div>
          </div>
        </div>
      ) : null}

      {onboarding ? <Setup state={state} notify={notify} /> : null}

      {prompt ? (
        <div className="dlprompt" role="dialog" aria-modal="true" aria-label={tr("downloadPromptTitle")}>
          <div className="dlcard">
            <div className="dlcardhead">
              <span className="dlcardicon" aria-hidden="true">
                <Icon name="download" size={16} />
              </span>
              <div>
                <div className="dlcardtitle">{tr("downloadPromptTitle")}</div>
                <div className="dlcardsub">{tr("downloadPromptText", { name: prompt.name })}</div>
              </div>
            </div>
            <div className="dlcardbody">
              <label className="dlfield">
                <span>{tr("saveTo")}</span>
                <div className="dlfolder">
                  <Icon name="folder" size={14} />
                  <input
                    value={saveFolder}
                    spellCheck={false}
                    autoComplete="off"
                    onChange={(event) => setSaveFolder(event.target.value)}
                  />
                  <button type="button" className="dlchoose" onClick={() => void chooseFolder()}>
                    {tr("chooseFolder")}
                  </button>
                </div>
              </label>
              <label className="dlcheck">
                <input
                  type="checkbox"
                  checked={alwaysHere}
                  onChange={(event) => setAlwaysHere(event.target.checked)}
                />
                <span>{tr("alwaysHere")}</span>
              </label>
            </div>
            <div className="dlcardfoot">
              <button type="button" className="btn-ghost" onClick={cancelDownload}>
                {tr("cancel")}
              </button>
              <button type="button" className="btn-primary" onClick={confirmDownload}>
                {tr("save")}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {dialog ? (
        <div className="dlprompt" role="dialog" aria-modal="true" aria-label={dialog.message}>
          <div className="dlcard">
            <div className="dlcardhead">
              <span className="dlcardicon" aria-hidden="true">
                <Icon name={dialog.kind === "alert" ? "info" : "help"} size={16} />
              </span>
              <div>
                <div className="dlcardtitle">{sourceOf(dialog.origin, state.origin)}</div>
                <div className="dlcardsub">
                  {dialog.kind === "confirm"
                    ? tr("dialogConfirm")
                    : dialog.kind === "prompt"
                      ? tr("dialogPrompt")
                      : tr("dialogAlert")}
                </div>
              </div>
            </div>
            <div className="dlcardbody">
              <p className="dlmessage">{dialog.message}</p>
              {dialog.kind === "prompt" ? (
                <label className="dlfield">
                  <span>{tr("dialogAnswer")}</span>
                  <div className="dlfolder">
                    <input
                      autoFocus
                      value={dialogText}
                      spellCheck={false}
                      autoComplete="off"
                      onChange={(event) => setDialogText(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") {
                          event.preventDefault();
                          answerDialog(true);
                        }
                      }}
                    />
                  </div>
                </label>
              ) : null}
            </div>
            <div className="dlcardfoot">
              {dialog.kind !== "alert" ? (
                <button type="button" className="btn-ghost" onClick={() => answerDialog(false)}>
                  {tr("cancel")}
                </button>
              ) : null}
              <button type="button" className="btn-primary" onClick={() => answerDialog(true)}>
                {tr("ok")}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {toast ? <div className="toast">{toast}</div> : null}
    </div>
  );
}

function WinControls({ maximized, lang }: { maximized: boolean; lang: Lang }) {
  const tr = (key: Key) => t(lang, key);
  return (
    <div className="wincontrols">
      <button
        className="wc"
        title={tr("minimize")}
        aria-label={tr("minimize")}
        onClick={() => void call("window_min")}
      >
        <Icon name="minimize" size={14} />
      </button>
      <button
        className="wc"
        title={maximized ? tr("restore") : tr("maximize")}
        aria-label={maximized ? tr("restore") : tr("maximize")}
        onClick={() => void call("window_max")}
      >
        <Icon name={maximized ? "restore" : "maximize"} size={13} />
      </button>
      <button
        className="wc closewin"
        title={tr("close")}
        aria-label={tr("closeDeeps")}
        onClick={() => void call("window_close")}
      >
        <Icon name="close" size={13} stroke={1.8} />
      </button>
    </div>
  );
}

function Setup({ state, notify }: { state: State; notify: (message: string) => void }) {
  const [step, setStep] = useState(0);
  const [language, setLanguage] = useState<Lang>(state.settings.language === "it" ? "it" : "en");
  const [engine, setEngine] = useState(state.settings.engine);
  const [layout, setLayout] = useState<TabLayout>(state.settings.tabLayout);
  const [shape, setShape] = useState<TabShape>(state.settings.tabShape);
  const [accent, setAccent] = useState(state.settings.accent);
  const [wallpaper, setWallpaper] = useState<Wallpaper>(state.settings.wallpaper);
  const [busy, setBusy] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importNote, setImportNote] = useState("");

  const tr = (key: Key, vars?: Record<string, string | number>) => t(language, key, vars);

  const engines = state.engines.length
    ? state.engines
    : [
        { key: "ddg", label: "DuckDuckGo", initial: "d" },
        { key: "google", label: "Google", initial: "g" },
        { key: "bing", label: "Bing", initial: "b" },
        { key: "startpage", label: "Startpage", initial: "s" },
      ];

  async function chooseLanguage(next: Lang) {
    setLanguage(next);
    await call("save_settings", { language: next });
  }

  async function finish(skip = false) {
    if (busy) return;
    setBusy(true);
    const reply = await call("save_settings", {
      onboarded: true,
      engine: skip ? state.settings.engine : engine,
      tabLayout: skip ? state.settings.tabLayout : layout,
      tabShape: skip ? state.settings.tabShape : shape,
      accent: skip ? state.settings.accent : accent,
      wallpaper: skip ? state.settings.wallpaper : wallpaper,
    });
    setBusy(false);
    if (!reply.ok) {
      notify(reply.error || tr("settingsFailed"));
    }
  }

  async function importNow() {
    if (importing) return;
    setImporting(true);
    const reply = await call<ImportReport>("import");
    setImporting(false);
    if (!reply.ok) {
      setImportNote(reply.error || tr("importFailed"));
      return;
    }
    const report = reply.data;
    if (!report || (!report.added && !report.skipped)) {
      setImportNote(tr("importNone"));
      return;
    }
    setImportNote(tr("importResult", { added: report.added, skipped: report.skipped }));
    notify(tr("importResult", { added: report.added, skipped: report.skipped }));
  }

  async function defaultNow() {
    const reply = await call<{ registered: boolean; path: string }>("default_browser");
    notify(reply.ok ? tr("defaultOpened") : reply.error || tr("defaultFailed"));
  }

  const steps = [tr("stepWelcome"), tr("stepSearch"), tr("stepLook"), tr("stepReady")];

  return (
    <div className="setup" role="dialog" aria-label={tr("setupAria")}>
      <div className="setup-card">
        <div className="setup-top">
          <div className="setup-brand">
            <Vortex size={26} />
            <span>deeps</span>
          </div>
          <div className="setup-dots" aria-hidden="true">
            {steps.map((label, index) => (
              <span key={label} className={index === step ? "on" : index < step ? "done" : ""} />
            ))}
          </div>
          <button className="setup-skip" onClick={() => void finish(true)} disabled={busy}>
            {tr("skip")}
          </button>
        </div>

        <div className="setup-body">
          {step === 0 ? (
            <div className="setup-step hero">
              <div className="setup-vortex">
                <Vortex size={72} />
              </div>
              <h1>{tr("welcomeTitle")}</h1>
              <p>{tr("welcomeText")}</p>
              <div className="setup-badges">
                <span><Icon name="layout" size={14} /> {tr("badgeVertical")}</span>
                <span><Icon name="translate" size={14} /> {tr("badgeTranslator")}</span>
                <span><Icon name="key" size={14} /> {tr("badgePasswords")}</span>
              </div>
              <div className="setup-label setup-language-label">{tr("chooseLanguage")}</div>
              <div className="language-row">
                <button
                  className={`language-card${language === "en" ? " on" : ""}`}
                  onClick={() => void chooseLanguage("en")}
                >
                  <span className="language-flag">EN</span>
                  <span>
                    <b>{tr("languageEn")}</b>
                    <em>English</em>
                  </span>
                  {language === "en" ? <Icon name="check" size={14} /> : null}
                </button>
                <button
                  className={`language-card${language === "it" ? " on" : ""}`}
                  onClick={() => void chooseLanguage("it")}
                >
                  <span className="language-flag">IT</span>
                  <span>
                    <b>{tr("languageIt")}</b>
                    <em>Italiano</em>
                  </span>
                  {language === "it" ? <Icon name="check" size={14} /> : null}
                </button>
              </div>
              <p className="setup-fine">{tr("languageHint")}</p>
            </div>
          ) : null}

          {step === 1 ? (
            <div className="setup-step">
              <h1>{tr("engineQuestion")}</h1>
              <p className="setup-sub">{tr("engineHint")}</p>
              <div className="engine-grid">
                {engines.map((item) => (
                  <button
                    key={item.key}
                    className={`engine-card${engine === item.key ? " on" : ""}`}
                    onClick={() => setEngine(item.key)}
                  >
                    <span className="engine-initial">!{item.initial}</span>
                    <span>{item.label}</span>
                    {engine === item.key ? <Icon name="check" size={14} /> : null}
                  </button>
                ))}
              </div>
            </div>
          ) : null}

          {step === 2 ? (
            <div className="setup-step">
              <h1>{tr("appearanceTitle")}</h1>
              <p className="setup-sub">{tr("appearanceHint")}</p>

              <div className="setup-label">{tr("layout")}</div>
              <div className="choice-row">
                <button className={`choice${layout === "top" ? " on" : ""}`} onClick={() => setLayout("top")}>
                  <span className="choice-preview top" />
                  {tr("horizontalLayout")}
                </button>
                <button className={`choice${layout === "vertical" ? " on" : ""}`} onClick={() => setLayout("vertical")}>
                  <span className="choice-preview vertical" />
                  {tr("verticalLayout")}
                </button>
              </div>

              <div className="setup-label">{tr("tabShape")}</div>
              <div className="choice-row compact">
                {(["rounded", "square", "pill"] as TabShape[]).map((key) => (
                  <button key={key} className={`shape-choice ${key}${shape === key ? " on" : ""}`} onClick={() => setShape(key)}>
                    <span />
                    {key === "rounded" ? tr("rounded") : key === "square" ? tr("square") : tr("pill")}
                  </button>
                ))}
              </div>

              <div className="setup-label">{tr("color")}</div>
              <div className="swatches">
                {ACCENTS.map((color) => (
                  <button
                    key={color}
                    className={`swatch${accent.toLowerCase() === color ? " on" : ""}`}
                    style={{ background: color }}
                    aria-label={`${tr("color")} ${color}`}
                    onClick={() => setAccent(color)}
                  />
                ))}
                <label className="swatch custom" title={tr("customColor")}>
                  <input type="color" value={accent} onChange={(event) => setAccent(event.target.value)} />
                </label>
              </div>

              <div className="setup-label">{tr("background")}</div>
              <div className="wall-grid">
                {WALLPAPERS.map((item) => (
                  <button
                    key={item.key}
                    className={`wall-card wall-${item.key}${wallpaper === item.key ? " on" : ""}`}
                    onClick={() => setWallpaper(item.key)}
                  >
                    <span />
                    {tr(item.label)}
                  </button>
                ))}
              </div>
            </div>
          ) : null}

          {step === 3 ? (
            <div className="setup-step done">
              <div className="setup-check"><Icon name="check" size={30} stroke={2.2} /></div>
              <h1>{tr("doneTitle")}</h1>
              <p>
                {tr("doneText", {
                  engine: engines.find((item) => item.key === engine)?.label ?? "DuckDuckGo",
                  layout: layout === "vertical" ? tr("verticalLayout").toLowerCase() : tr("horizontalLayout").toLowerCase(),
                  wallpaper: tr(WALLPAPERS.find((item) => item.key === wallpaper)?.label ?? "wallNone").toLowerCase(),
                })}
              </p>
              <div className="setup-summary">
                <span><Icon name="search" size={14} /> {tr("summaryBangs")}</span>
                <span><Icon name="vpn" size={14} /> {tr("summaryVpn")}</span>
                <span><Icon name="key" size={14} /> {tr("summaryLogin")}</span>
                <span><Icon name="bookmark" size={14} /> {tr("summaryImport")}</span>
              </div>
              <div className="setup-actions">
                <button className="btn-ghost" onClick={() => void importNow()} disabled={importing}>
                  {importing ? tr("importing") : tr("importBookmarks")}
                </button>
                <button className="btn-ghost" onClick={() => void defaultNow()}>
                  {tr("setDefault")}
                </button>
              </div>
              {importNote ? <p className="setup-fine">{importNote}</p> : null}
            </div>
          ) : null}
        </div>

        <div className="setup-foot">
          <button className="btn-ghost" onClick={() => setStep(Math.max(0, step - 1))} disabled={step === 0 || busy}>
            {tr("backLabel")}
          </button>
          <span className="setup-stepname">{steps[step]}</span>
          {step < 3 ? (
            <button className="btn-primary" onClick={() => setStep(Math.min(3, step + 1))} disabled={busy}>
              {tr("next")}
            </button>
          ) : (
            <button className="btn-primary" onClick={() => void finish(false)} disabled={busy}>
              {busy ? tr("saving") : tr("startBrowsing")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function MenuRow({
  icon,
  label,
  kbd,
  detail,
  onClick,
}: {
  icon: IconName;
  label: string;
  kbd?: string;
  detail?: string;
  onClick: () => void;
}) {
  return (
    <button className="menuitem" onClick={onClick}>
      <span className="menuicon">
        <Icon name={icon} size={15} />
      </span>
      <span className="menulabel">
        {label}
        {detail ? <em>{detail}</em> : null}
      </span>
      {kbd ? <kbd>{kbd}</kbd> : null}
    </button>
  );
}
