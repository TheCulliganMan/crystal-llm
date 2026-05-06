"use client";

import { ReactNode } from "react";
import { usePathname } from "next/navigation";
import { MainNav } from "@/components/layout/main-nav";
import { TopBar } from "@/components/layout/top-bar";

type RouteShellProps = {
  children: ReactNode;
};

const DESKTOP_ROUTE = "/desktop";

export const RouteShell = ({ children }: RouteShellProps) => {
  const pathname = usePathname();
  const isPlayRoute = pathname === "/";

  if (pathname === DESKTOP_ROUTE) {
    return (
      <main className="desktop-page-shell h-dvh min-h-dvh w-full overflow-hidden bg-black">
        {children}
      </main>
    );
  }

  return (
    <div className="kc-page-shell bg-transparent">
      <main className="mx-auto flex min-h-screen w-full max-w-[var(--content-max-width)] items-start gap-2">
        <MainNav mode="desktop" />
        <div className={`min-w-0 flex-1 ${isPlayRoute ? "pb-[calc(var(--nav-mobile-height)+0.35rem)] md:pb-0" : "pb-20 md:pb-0"}`}>
          <TopBar />
          <section className={`min-w-0 ${isPlayRoute ? "px-0 py-0 md:px-4 md:py-3" : "px-3 py-2 md:px-4 md:py-3"}`}>{children}</section>
        </div>
        <MainNav mode="mobile" />
      </main>
    </div>
  );
};
