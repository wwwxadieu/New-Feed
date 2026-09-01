interface IconProps {
  size?: number;
}

const base = (size: number) => ({
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.7,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
});

export const SearchIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <circle cx="11" cy="11" r="6.5" />
    <path d="m20 20-4.2-4.2" />
  </svg>
);

export const RefreshIcon = ({ size = 16 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M20 11a8 8 0 1 0-2.3 6" />
    <path d="M20 5v6h-6" />
  </svg>
);

export const PlusIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M12 5v14M5 12h14" />
  </svg>
);

export const CloseIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M6 6l12 12M18 6 6 18" />
  </svg>
);

export const MinimizeIcon = ({ size = 14 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M6 12h12" />
  </svg>
);

export const MaximizeIcon = ({ size = 13 }: IconProps) => (
  <svg {...base(size)}>
    <rect x="5.5" y="5.5" width="13" height="13" rx="2.5" />
  </svg>
);

export const TrashIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M4.5 7h15M9.5 7V5.5A1.5 1.5 0 0 1 11 4h2a1.5 1.5 0 0 1 1.5 1.5V7" />
    <path d="M6.5 7l.8 11.2A1.8 1.8 0 0 0 9.1 20h5.8a1.8 1.8 0 0 0 1.8-1.8L17.5 7" />
  </svg>
);

export const ExternalIcon = ({ size = 14 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M14 4h6v6" />
    <path d="M20 4 11 13" />
    <path d="M18 14.5V18a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h3.5" />
  </svg>
);

export const SunIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.2 5.2l1.4 1.4M17.4 17.4l1.4 1.4M18.8 5.2l-1.4 1.4M6.6 17.4l-1.4 1.4" />
  </svg>
);

export const MoonIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M20 14.2A8.2 8.2 0 0 1 9.8 4a8.2 8.2 0 1 0 10.2 10.2Z" />
  </svg>
);

export const AutoIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <circle cx="12" cy="12" r="8" />
    <path d="M12 4a8 8 0 0 1 0 16Z" fill="currentColor" stroke="none" />
  </svg>
);

/** Ngọn lửa — tin đang được nhiều báo cùng đưa. */
export const FlameIcon = ({ size = 13 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M12 2.8c2.7 3 4.1 5.3 4.1 7a4.1 4.1 0 0 1-8.2 0c0-.9.3-1.8.9-2.7.5 1 1.1 1.6 1.8 1.9-.2-2.2.3-4.3 1.4-6.2Z" />
    <path d="M12 21.2a6.6 6.6 0 0 0 6.6-6.6c0-1-.2-2-.6-2.9" />
    <path d="M5.4 14.6A6.6 6.6 0 0 0 12 21.2" />
  </svg>
);

export const SourceIcon = ({ size = 15 }: IconProps) => (
  <svg {...base(size)}>
    <path d="M4 6.5h11M4 12h11M4 17.5h7" />
    <circle cx="18.5" cy="17.5" r="1.6" fill="currentColor" stroke="none" />
  </svg>
);
