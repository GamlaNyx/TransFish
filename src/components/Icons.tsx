interface IconProps {
  size?: number;
  className?: string;
}

const stroke = {
  fill: "none" as const,
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

/** `slider.horizontal.3` — SF Symbols settings glyph. */
export function SettingsIcon({ size = 18, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} aria-hidden>
      <path d="M3.5 7h5.2M12.7 7h7.8M3.5 12h8.4M15.9 12h4.6M3.5 17h3.6M11.1 17h9.4" {...stroke} />
      <circle cx="10.7" cy="7" r="2" fill="currentColor" stroke="none" />
      <circle cx="13.9" cy="12" r="2" fill="currentColor" stroke="none" />
      <circle cx="9.1" cy="17" r="2" fill="currentColor" stroke="none" />
    </svg>
  );
}

/** `pin.fill` — used solid, tinted by its active state. */
export function PinIcon({ size = 18, className }: IconProps & { active?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} aria-hidden>
      <path d="M12 17v5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      <path
        d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1v3.76Z"
        fill="currentColor"
      />
    </svg>
  );
}

/** `doc.on.doc` */
export function CopyIcon({ size = 15, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} aria-hidden>
      <rect x="8.5" y="8.5" width="11" height="11" rx="2.6" {...stroke} />
      <path d="M15.4 5.4a2.4 2.4 0 0 0-2.3-1.6H6.9a2.6 2.6 0 0 0-2.6 2.6v6.2c0 1 .6 1.9 1.6 2.3" {...stroke} />
    </svg>
  );
}

export function CheckIcon({ size = 14, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} aria-hidden>
      <path d="m5 12.5 4.3 4.3L19 7.2" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

/** `exclamationmark.circle` */
export function AlertIcon({ size = 16, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} aria-hidden>
      <circle cx="12" cy="12" r="9" {...stroke} />
      <path d="M12 7.6v5.2" {...stroke} />
      <circle cx="12" cy="16.4" r="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

/** `chevron.left` */
export function ChevronLeftIcon({ size = 18, className }: IconProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" className={className} aria-hidden>
      <path d="M14.6 5.6 8.2 12l6.4 6.4" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
