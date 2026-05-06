"use client";

import type { Session, SupabaseClient } from "@supabase/supabase-js";
import type { ReactNode } from "react";
import { createContext, useContext, useEffect, useMemo, useState } from "react";
import { createSupabaseBrowserClient } from "@/lib/supabase/browser";
import { isSupabaseConfigured } from "@/lib/supabase/env";
import type { Database } from "@/lib/supabase/types";

type SupabaseContextValue = {
  supabaseClient: SupabaseClient<Database> | null;
  session: Session | null;
  isConfigured: boolean;
};

const SupabaseContext = createContext<SupabaseContextValue | null>(null);

export const SupabaseProvider = ({
  children,
  session,
}: {
  children: ReactNode;
  session?: Session | null;
}) => {
  const [supabaseClient] = useState(() => createSupabaseBrowserClient());
  const [currentSession, setCurrentSession] = useState<Session | null>(session ?? null);
  const [configured] = useState(() => isSupabaseConfigured());

  useEffect(() => {
    setCurrentSession(session ?? null);
  }, [session]);

  useEffect(() => {
    if (!supabaseClient) {
      return;
    }
    let active = true;
    supabaseClient.auth
      .getSession()
      .then(({ data }) => {
        if (active) {
          setCurrentSession(data.session ?? null);
        }
      })
      .catch(() => undefined);
    const { data } = supabaseClient.auth.onAuthStateChange((_event, nextSession) => {
      setCurrentSession(nextSession);
    });
    return () => {
      active = false;
      data.subscription.unsubscribe();
    };
  }, [supabaseClient]);

  const value = useMemo(
    () => ({ supabaseClient, session: currentSession, isConfigured: configured }),
    [supabaseClient, currentSession, configured]
  );

  return <SupabaseContext.Provider value={value}>{children}</SupabaseContext.Provider>;
};

export const useSupabase = () => {
  const context = useContext(SupabaseContext);
  if (!context) {
    throw new Error("SupabaseProvider is missing from the component tree.");
  }
  return context;
};
