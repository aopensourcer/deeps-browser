const rings = [
  { rx: 27, ry: 17.4, width: 3.6, opacity: 0.97, dash: "81 19", offset: -2 },
  { rx: 22.4, ry: 14.4, width: 3.1, opacity: 0.85, dash: "79 21", offset: -12 },
  { rx: 18.1, ry: 11.6, width: 2.6, opacity: 0.71, dash: "77 23", offset: -22 },
  { rx: 14.1, ry: 9.1, width: 2.1, opacity: 0.57, dash: "75 25", offset: -32 },
  { rx: 10.5, ry: 6.8, width: 1.7, opacity: 0.43, dash: "73 27", offset: -42 },
  { rx: 7.3, ry: 4.7, width: 1.3, opacity: 0.3, dash: "71 29", offset: -52 },
];

export function Vortex({
  size = 18,
  spin = false,
  className = "",
}: {
  size?: number;
  spin?: boolean;
  className?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      fill="none"
      className={spin ? `vortex spin ${className}`.trim() : `vortex ${className}`.trim()}
      aria-hidden="true"
      focusable="false"
    >
      <g transform="rotate(-22 32 32)" stroke="currentColor" strokeLinecap="round">
        {rings.map((ring) => (
          <ellipse
            key={ring.rx}
            cx="32"
            cy="32"
            rx={ring.rx}
            ry={ring.ry}
            strokeWidth={ring.width}
            opacity={ring.opacity}
            pathLength={100}
            strokeDasharray={ring.dash}
            strokeDashoffset={ring.offset}
          />
        ))}
      </g>
      <ellipse cx="32" cy="32" rx="5.1" ry="3.3" fill="var(--logo-hole, #08080a)" />
    </svg>
  );
}
