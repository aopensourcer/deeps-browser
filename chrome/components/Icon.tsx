import type { ReactNode } from "react";

export type IconName =
  | "back"
  | "forward"
  | "reload"
  | "close"
  | "plus"
  | "lock"
  | "globe"
  | "search"
  | "star"
  | "starFilled"
  | "puzzle"
  | "history"
  | "bookmark"
  | "key"
  | "settings"
  | "power"
  | "minimize"
  | "maximize"
  | "restore"
  | "translate"
  | "vpn"
  | "palette"
  | "layout"
  | "pin"
  | "check"
  | "info"
  | "help"
  | "download"
  | "folder"
  | "trash";

const shapes: Record<IconName, ReactNode> = {
  back: <path d="m14.5 4.5-7.5 7.5 7.5 7.5" />,
  forward: <path d="m9.5 4.5 7.5 7.5-7.5 7.5" />,
  reload: (
    <>
      <path d="M21 12a9 9 0 1 1-2.64-6.36" />
      <path d="M21 4v5h-5" />
    </>
  ),
  close: <path d="M6 6l12 12M18 6 6 18" />,
  plus: <path d="M12 5v14M5 12h14" />,
  lock: (
    <>
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V8a4 4 0 0 1 8 0v3" />
    </>
  ),
  globe: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17" />
      <path d="M12 3.5a15 15 0 0 1 0 17a15 15 0 0 1 0-17" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.2-3.2" />
    </>
  ),
  star: (
    <path d="m12 3.6 2.55 5.17 5.7.83-4.13 4.02.98 5.68L12 16.62l-5.1 2.68.98-5.68L3.75 9.6l5.7-.83z" />
  ),
  starFilled: (
    <path
      d="m12 3.6 2.55 5.17 5.7.83-4.13 4.02.98 5.68L12 16.62l-5.1 2.68.98-5.68L3.75 9.6l5.7-.83z"
      fill="currentColor"
    />
  ),
  puzzle: (
    <path d="M9 4a2 2 0 0 1 4 0v1h4v4h1a2 2 0 0 1 0 4h-1v4h-4v-1a2 2 0 0 0-4 0v1H5v-4h1a2 2 0 0 0 0-4H5V5h4z" />
  ),
  history: (
    <>
      <path d="M3.2 12a8.8 8.8 0 1 0 2.9-6.5" />
      <path d="M3 4v5h5" />
      <path d="M12 8v4.2l2.8 1.8" />
    </>
  ),
  bookmark: <path d="M6 4h12v17l-6-4.2L6 21z" />,
  key: (
    <>
      <circle cx="8" cy="15" r="3.5" />
      <path d="m10.5 12.5 8-8" />
      <path d="m16 7 2 2" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3.2" />
      <path d="M12 2.8v2.6M12 18.6v2.6M2.8 12h2.6M18.6 12h2.6M5.5 5.5l1.9 1.9M16.6 16.6l1.9 1.9M18.5 5.5l-1.9 1.9M7.4 16.6l-1.9 1.9" />
    </>
  ),
  power: (
    <>
      <path d="M12 3v9" />
      <path d="M6.3 6.3a8 8 0 1 0 11.4 0" />
    </>
  ),
  minimize: <path d="M5 12h14" />,
  maximize: <rect x="5.5" y="5.5" width="13" height="13" rx="1.5" />,
  restore: (
    <>
      <rect x="4.5" y="8" width="11" height="11" rx="1.5" />
      <path d="M8 8V5.5A1.5 1.5 0 0 1 9.5 4h9A1.5 1.5 0 0 1 20 5.5v9a1.5 1.5 0 0 1-1.5 1.5H16" />
    </>
  ),
  translate: (
    <>
      <path d="M4 5h9M8.5 3v2M6 5c0 4 2.5 7 6 8.5" />
      <path d="M11.5 5c0 4-2.5 6.8-6 8" />
      <path d="m13 20 3.5-9 3.5 9M14.3 17h4.4" />
    </>
  ),
  vpn: (
    <>
      <path d="M12 3.2 19 6v5.4c0 4.4-2.9 7.7-7 9.4-4.1-1.7-7-5-7-9.4V6z" />
      <path d="m9.2 12 2 2 3.8-4" />
    </>
  ),
  palette: (
    <>
      <path d="M12 3.5a8.5 8.5 0 1 0 0 17c1.4 0 2-.9 2-1.9 0-.8-.5-1.2-.5-2 0-1.1.9-2 2-2h1.5c2 0 3.5-1.5 3.5-3.5A8.5 8.5 0 0 0 12 3.5z" />
      <circle cx="7.5" cy="10" r="1" fill="currentColor" stroke="none" />
      <circle cx="10.5" cy="7" r="1" fill="currentColor" stroke="none" />
      <circle cx="15" cy="7.5" r="1" fill="currentColor" stroke="none" />
    </>
  ),
  layout: (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
      <path d="M9 4.5v15" />
    </>
  ),
  pin: (
    <>
      <path d="M9 4h6l-1 5.5 3 3V15H7v-2.5l3-3z" />
      <path d="M12 15v5" />
    </>
  ),
  download: (
    <>
      <path d="M12 4v10" />
      <path d="m7.5 9.5 4.5 4.5 4.5-4.5" />
      <path d="M5 19h14" />
    </>
  ),
  check: <path d="m5 12.5 4.2 4.2L19 6.5" />,
  info: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 11v5" />
      <circle cx="12" cy="8" r="0.9" fill="currentColor" stroke="none" />
    </>
  ),
  help: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M9.4 9.4a2.7 2.7 0 1 1 3.6 2.5c-.7.3-1 .9-1 1.6v.4" />
      <circle cx="12" cy="17" r="0.9" fill="currentColor" stroke="none" />
    </>
  ),
  folder: (
    <path d="M3.5 7.2a2 2 0 0 1 2-2h3.6l2 2.4h7.4a2 2 0 0 1 2 2v7.2a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" />
  ),
  trash: (
    <>
      <path d="M4.5 6.5h15" />
      <path d="M9 6.5V4.8h6v1.7" />
      <path d="M6.5 6.5 7.4 20h9.2l.9-13.5" />
      <path d="M10.2 10v6M13.8 10v6" />
    </>
  ),
};

export function Icon({
  name,
  size = 17,
  stroke = 1.7,
}: {
  name: IconName;
  size?: number;
  stroke?: number;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={stroke}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {shapes[name]}
    </svg>
  );
}
