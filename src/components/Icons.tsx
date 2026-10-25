interface IconProps {
  size?: number;
  className?: string;
}

const base = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

export function GearIcon({ size = 15, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" className={className} aria-hidden>
      <path d="M2.6 4.6h5.5M11 4.6h2.4M2.6 8h1.2M6.9 8h6.5M2.6 11.4h7.1M12.9 11.4h1.1" {...base} />
      <circle cx="9.55" cy="4.6" r="1.45" fill="currentColor" stroke="none" />
      <circle cx="5.25" cy="8" r="1.45" fill="currentColor" stroke="none" />
      <circle cx="11.25" cy="11.4" r="1.45" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function CopyIcon({ size = 13, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" className={className} aria-hidden>
      <rect x="5.3" y="5.3" width="7.7" height="7.7" rx="1.6" {...base} />
      <path d="M10.4 3.2a1.6 1.6 0 0 0-1.6-1.2H4.2a1.6 1.6 0 0 0-1.6 1.6v4.7c0 .7.4 1.3 1 1.5" {...base} />
    </svg>
  );
}

export function CheckIcon({ size = 13, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" className={className} aria-hidden>
      <path d="m3.4 8.5 2.9 2.9 6.3-6.6" {...base} />
    </svg>
  );
}

export function AlertIcon({ size = 14, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" className={className} aria-hidden>
      <circle cx="8" cy="8" r="6.2" {...base} />
      <path d="M8 5.1v3.6" {...base} />
      <circle cx="8" cy="11.1" r="0.65" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function ArrowLeftIcon({ size = 15, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" className={className} aria-hidden>
      <path d="M12.6 8H3.6M7 3.8 2.8 8 7 12.2" {...base} />
    </svg>
  );
}

export function SealMark({ size = 21 }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden>
      <rect x="1" y="1" width="30" height="30" rx="7.6" fill="#B23A2A" />
      <rect
        x="3.6"
        y="3.6"
        width="24.8"
        height="24.8"
        rx="5.4"
        fill="none"
        stroke="#F7F2E6"
        strokeOpacity="0.26"
        strokeWidth="0.7"
      />
      <path
        d="M9 16.5c2.9-3.9 6.8-5.4 10.5-4.4 2 .5 3.6 1.9 4.6 3.5-.9 2-2.4 3.5-4.5 4.1-3.8 1.1-7.7-.3-10.6-3.2Z"
        fill="#F7F2E6"
      />
      <path d="m9.2 16.2-3.1-3v6.2l3.1-2.6Z" fill="#F7F2E6" />
      <path d="M17.8 12.4c-.8 2.3-.8 5.6 0 7.9" stroke="#B23A2A" strokeWidth="0.9" fill="none" opacity="0.55" />
      <circle cx="21.1" cy="15.6" r="0.92" fill="#B23A2A" />
    </svg>
  );
}
