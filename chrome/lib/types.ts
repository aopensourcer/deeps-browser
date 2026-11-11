export type Tab = {
  id: number;
  title: string;
  url: string;
  loading: boolean;
  icon?: string | null;
};

export type EngineOption = {
  key: string;
  label: string;
  initial: string;
};

export type SiteOption = {
  key: string;
  label: string;
};

export type TabLayout = "top" | "vertical";
export type TabShape = "rounded" | "square" | "pill";
export type Wallpaper = "none" | "aurora" | "mesh" | "grid" | "dunes" | "stars";
export type Language = "en" | "it";

export type Settings = {
  engine: string;
  engineLabel: string;
  shortcuts: boolean;
  adblock: boolean;
  tabLayout: TabLayout;
  tabShape: TabShape;
  accent: string;
  wallpaper: Wallpaper;
  saveLogins: boolean;
  language: Language;
  translateFrom: string;
  translateTo: string;
  onboarded: boolean;
  downloadDir: string;
  askDownloadDir: boolean;
  hasPhoto: boolean;
};

export type ImportReport = {
  added: number;
  skipped: number;
  sources: { name: string; found: number; added: number }[];
};

export type DownloadEntry = {
  id: number;
  url: string;
  name: string;
  path: string;
  /** `running`, `done` or `failed`. */
  state: string;
  started: number;
  finished: number;
  /** Bytes received and total size (0 when the server does not report it). */
  received: number;
  total: number;
};

/** A native download waiting for the user to pick a folder. */
export type DownloadPrompt = {
  id: number;
  url: string;
  name: string;
  /** Folder Deeps suggests by default. */
  folder: string;
};

/** A page `alert` / `confirm` / `prompt` waiting for the Deeps card. */
export type ScriptDialog = {
  id: number;
  /** `alert`, `confirm` or `prompt`. */
  kind: string;
  message: string;
  /** Initial text of a `prompt`. */
  defaultText: string;
  /** Page that opened the dialog, usually the host. */
  origin: string;
};

export type State = {
  tabs: Tab[];
  active: number;
  bookmarked: boolean;
  maximized: boolean;
  overlay: boolean;
  /** True while the vertical sidebar is collapsed to favicons. */
  sidebarCollapsed: boolean;
  downloads: DownloadEntry[];
  /** True while the minimized downloads panel is expanded. */
  downloadsOpen: boolean;
  /** Native downloads waiting for a destination. */
  downloadPrompt: DownloadPrompt[];
  /** Page dialogs waiting for an answer. */
  dialogs: ScriptDialog[];
  /** Absolute loopback URL of the custom wallpaper photo, or "". */
  photoUrl: string;
  /** Origin of the internal loopback server, e.g. http://127.0.0.1:52755 */
  origin: string;
  engines: EngineOption[];
  sites?: SiteOption[];
  settings: Settings;
};

export type Reply<T = unknown> = {
  ok: boolean;
  data?: T;
  error?: string;
};
