"use client";

import Link from "next/link";
import type { AnchorHTMLAttributes } from "react";

type DaisyLinkButtonVariant = "contained" | "outlined" | "ghost" | "link" | "neutral" | "primary";

type LinkButtonProps = Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> & {
  href: string;
  target?: string;
  rel?: string;
  variant?: DaisyLinkButtonVariant;
};

const isExternalHref = (href: string): boolean => /^https?:\/\//i.test(href);

const VARIANT_CLASSES: Record<DaisyLinkButtonVariant, string> = {
  contained: "btn-primary",
  outlined: "btn-outline",
  ghost: "btn-ghost",
  link: "btn-link",
  neutral: "btn-neutral",
  primary: "btn-primary",
};

export const LinkButton = ({
  href,
  target,
  rel,
  variant,
  className,
  children,
  ...props
}: LinkButtonProps) => {
  const isExternal = Boolean(target) || isExternalHref(href);
  const buttonClass = ["btn", variant ? VARIANT_CLASSES[variant] : null, className]
    .filter(Boolean)
    .join(" ");

  if (isExternal) {
    return (
      <a
        href={href}
        target={target ?? "_blank"}
        rel={rel ?? "noopener noreferrer"}
        className={buttonClass}
        {...props}
      >
        {children}
      </a>
    );
  }

  return (
    <Link href={href} className={buttonClass} {...props}>
      {children}
    </Link>
  );
};
