import * as React from "react";

type Props = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost";
  size?: "sm" | "md" | "lg";
  href?: string;
};

export function Button({ variant = "primary", size = "md", href, className = "", children, ...rest }: Props) {
  const base =
    "inline-flex items-center justify-center font-medium tracking-[-0.01em] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--fg)] focus-visible:ring-offset-2 disabled:opacity-50 disabled:pointer-events-none";
  const sizes = {
    sm: "h-8 px-3 text-[13px] rounded-[8px]",
    md: "h-9 px-4 text-[13.5px] rounded-[10px]",
    lg: "h-10 px-5 text-[14px] rounded-[11px]",
  }[size];
  const variants = {
    primary: "bg-[var(--fg)] text-white hover:bg-[var(--fg-2)] border border-transparent shadow-[0_1px_2px_rgba(0,0,0,0.06)]",
    secondary: "bg-white text-[var(--fg)] border border-[var(--border)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-2)]",
    ghost: "bg-transparent text-[var(--muted)] hover:text-[var(--fg)] hover:bg-[var(--bg-2)]",
  }[variant];

  const cls = `${base} ${sizes} ${variants} ${className}`;

  if (href) {
    const anchorProps = rest as unknown as React.AnchorHTMLAttributes<HTMLAnchorElement>;
    return (
      <a href={href} className={cls} {...anchorProps}>
        {children}
      </a>
    );
  }
  return (
    <button className={cls} {...rest}>
      {children}
    </button>
  );
}
