"use client";

import { useEffect, useState } from "react";

type CopySessionUrlButtonProps = {
  sessionUrl: string;
};

const COPY_RESET_MS = 1400;

const resolveAbsoluteSessionUrl = (sessionUrl: string): string => {
  if (/^https?:\/\//i.test(sessionUrl)) {
    return sessionUrl;
  }
  if (typeof window === "undefined") {
    return sessionUrl;
  }
  return new URL(sessionUrl, window.location.origin).toString();
};

export const CopySessionUrlButton = ({ sessionUrl }: CopySessionUrlButtonProps) => {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) {
      return;
    }
    const timer = window.setTimeout(() => setCopied(false), COPY_RESET_MS);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const handleCopy = async () => {
    if (typeof navigator === "undefined" || !navigator.clipboard) {
      return;
    }
    try {
      await navigator.clipboard.writeText(resolveAbsoluteSessionUrl(sessionUrl));
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  return (
    <button type="button" className="btn btn-sm btn-primary" onClick={handleCopy}>
      {copied ? "Copied" : "Copy URL"}
    </button>
  );
};
