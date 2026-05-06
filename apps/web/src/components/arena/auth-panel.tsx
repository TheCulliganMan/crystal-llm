"use client";

import { type ChangeEvent, useEffect, useState, type FormEvent, useId } from "react";
import type { Provider } from "@supabase/supabase-js";
import { useSupabase } from "@/components/providers/supabase-provider";
import { buildAuthCallbackUrl } from "@/lib/supabase/urls";

type AuthMode = "password" | "magic" | "signup" | "reset";
type AuthAction =
  | "sign-in"
  | "sign-up"
  | "magic-link"
  | "reset"
  | "oauth"
  | "sign-out"
  | "password-update";

type StatusMessage = {
  text: string;
  severity: "success" | "info" | "warning" | "error";
};

const oauthProviders = (process.env.NEXT_PUBLIC_SUPABASE_OAUTH_PROVIDERS ?? "")
  .split(",")
  .map((provider) => provider.trim())
  .filter(Boolean);

const toProviderLabel = (provider: string) =>
  provider.length ? `${provider[0].toUpperCase()}${provider.slice(1)}` : provider;

const resolveRedirectUrl = () => {
  if (typeof window === "undefined") {
    return undefined;
  }
  const nextPath = `${window.location.pathname}${window.location.search}`;
  return buildAuthCallbackUrl(window.location.origin, nextPath);
};

const ALERT_VARIANT_CLASS: Record<StatusMessage["severity"], string> = {
  success: "alert-success",
  info: "alert-info",
  warning: "alert-warning",
  error: "alert-error",
};

const FormField = ({
  label,
  required,
  value,
  onChange,
  type = "text",
  placeholder,
  disabled,
  autoComplete,
}: {
  label: string;
  required?: boolean;
  value: string;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
  type?: string;
  placeholder?: string;
  disabled?: boolean;
  autoComplete?: string;
}) => (
  <label className="form-control">
    <span className="label label-text">
      {label}
      {required && <span aria-hidden="true" className="text-error ml-1">*</span>}
    </span>
    <input
      className="input input-bordered w-full"
      type={type}
      value={value}
      required={required}
      onChange={onChange}
      placeholder={placeholder}
      disabled={disabled}
      autoComplete={autoComplete}
    />
  </label>
);

const AlertBanner = ({ message }: { message: StatusMessage }) => (
  <div role="alert" className={`alert text-sm ${ALERT_VARIANT_CLASS[message.severity]}`}>
    {message.text}
  </div>
);

export const AuthPanel = () => {
  const { supabaseClient, session, isConfigured } = useSupabase();
  const [mode, setMode] = useState<AuthMode>("password");
  const tabIdPrefix = useId();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmNewPassword, setConfirmNewPassword] = useState("");
  const [message, setMessage] = useState<StatusMessage | null>(null);
  const [loading, setLoading] = useState<AuthAction | null>(null);

  useEffect(() => {
    setMessage(null);
  }, [mode]);

  const startAction = (action: AuthAction) => {
    setLoading(action);
    setMessage(null);
  };

  const finishAction = () => {
    setLoading(null);
  };

  const runAction = async (action: AuthAction, fn: () => Promise<void>) => {
    startAction(action);
    try {
      await fn();
    } finally {
      finishAction();
    }
  };

  const handleSignOut = async () => {
    if (!supabaseClient) {
      return;
    }
    await runAction("sign-out", async () => {
      const { error } = await supabaseClient.auth.signOut();
      if (error) {
        setMessage({ text: error.message, severity: "error" });
      } else {
        setMessage({ text: "Signed out successfully.", severity: "success" });
      }
    });
  };

  const handleSignIn = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!supabaseClient) {
      setMessage({ text: "Account services are currently unavailable.", severity: "warning" });
      return;
    }
    await runAction("sign-in", async () => {
      const { error } = await supabaseClient.auth.signInWithPassword({
        email,
        password,
      });
      if (error) {
        setMessage({ text: error.message, severity: "error" });
      } else {
        setMessage({ text: "Signed in successfully.", severity: "success" });
      }
    });
  };

  const handleMagicLink = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!supabaseClient) {
      setMessage({ text: "Account services are currently unavailable.", severity: "warning" });
      return;
    }
    await runAction("magic-link", async () => {
      const { error } = await supabaseClient.auth.signInWithOtp({
        email,
        options: { emailRedirectTo: resolveRedirectUrl() },
      });
      if (error) {
        setMessage({ text: error.message, severity: "error" });
      } else {
        setMessage({ text: "Check your email for a sign-in link.", severity: "info" });
      }
    });
  };

  const handleSignUp = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!supabaseClient) {
      setMessage({ text: "Account services are currently unavailable.", severity: "warning" });
      return;
    }
    if (password !== confirmPassword) {
      setMessage({ text: "Passwords do not match.", severity: "error" });
      return;
    }
    await runAction("sign-up", async () => {
      const { data, error } = await supabaseClient.auth.signUp({
        email,
        password,
        options: { emailRedirectTo: resolveRedirectUrl() },
      });
      if (error) {
        setMessage({ text: error.message, severity: "error" });
      } else if (!data?.session) {
        setMessage({ text: "Check your email to confirm your account.", severity: "info" });
      } else {
        setMessage({ text: "Account created successfully.", severity: "success" });
      }
    });
  };

  const handleResetPassword = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!supabaseClient) {
      setMessage({ text: "Account services are currently unavailable.", severity: "warning" });
      return;
    }
    await runAction("reset", async () => {
      const { error } = await supabaseClient.auth.resetPasswordForEmail(email, {
        redirectTo: resolveRedirectUrl(),
      });
      if (error) {
        setMessage({ text: error.message, severity: "error" });
      } else {
        setMessage({ text: "Password reset email sent.", severity: "info" });
      }
    });
  };

  const handleOAuth = async (provider: string) => {
    if (!supabaseClient) {
      setMessage({ text: "Account services are currently unavailable.", severity: "warning" });
      return;
    }
    await runAction("oauth", async () => {
      const { error } = await supabaseClient.auth.signInWithOAuth({
        provider: provider as Provider,
        options: { redirectTo: resolveRedirectUrl() },
      });
      if (error) {
        setMessage({ text: error.message, severity: "error" });
      }
    });
  };

  const handleUpdatePassword = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!supabaseClient) {
      setMessage({ text: "Account services are currently unavailable.", severity: "warning" });
      return;
    }
    if (newPassword !== confirmNewPassword) {
      setMessage({ text: "Passwords do not match.", severity: "error" });
      return;
    }
    await runAction("password-update", async () => {
      const { error } = await supabaseClient.auth.updateUser({ password: newPassword });
      if (error) {
        setMessage({ text: error.message, severity: "error" });
      } else {
        setMessage({ text: "Password updated successfully.", severity: "success" });
        setNewPassword("");
        setConfirmNewPassword("");
      }
    });
  };

  if (session?.user) {
    return (
      <section className="space-y-2">
        <p className="text-sm text-base-content">Signed in as {session.user.email}</p>
        <p className="text-xs text-base-content/70">User ID: {session.user.id}</p>
        {message ? <AlertBanner message={message} /> : null}
        <div className="divider" />
        <h3 className="text-sm font-semibold text-base-content">Change password</h3>
        <form className="space-y-2" onSubmit={handleUpdatePassword}>
          <FormField
            type="password"
            label="New password"
            value={newPassword}
            onChange={(event) => setNewPassword(event.target.value)}
            disabled={!isConfigured || loading !== null}
            autoComplete="new-password"
          />
          <FormField
            type="password"
            label="Confirm password"
            value={confirmNewPassword}
            onChange={(event) => setConfirmNewPassword(event.target.value)}
            disabled={!isConfigured || loading !== null}
            autoComplete="new-password"
          />
          <button
            type="submit"
            className="btn btn-sm btn-primary"
            disabled={!isConfigured || loading !== null}
          >
            {loading === "password-update" ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Update password
          </button>
        </form>
        <div className="divider" />
        <button
          type="button"
          className="btn btn-sm btn-outline"
          onClick={handleSignOut}
          disabled={loading !== null}
        >
          {loading === "sign-out" ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
          Sign out
        </button>
      </section>
    );
  }

  return (
    <section className="space-y-2">
      {!isConfigured ? (
        <AlertBanner message={{ text: "Account services are currently unavailable. Please try again shortly.", severity: "warning" }} />
      ) : null}
      {message ? <AlertBanner message={message} /> : null}
      <div className="tabs tabs-boxed bg-base-200 p-1" role="tablist" aria-label="Authentication modes">
        <button
          id={`${tabIdPrefix}-tab-password`}
          type="button"
          role="tab"
          aria-controls={`${tabIdPrefix}-panel-password`}
          aria-selected={mode === "password"}
          className={`tab ${mode === "password" ? "tab-active" : ""}`}
          onClick={() => setMode("password")}
        >
          Password
        </button>
        <button
          id={`${tabIdPrefix}-tab-magic`}
          type="button"
          role="tab"
          aria-controls={`${tabIdPrefix}-panel-magic`}
          aria-selected={mode === "magic"}
          className={`tab ${mode === "magic" ? "tab-active" : ""}`}
          onClick={() => setMode("magic")}
        >
          Magic link
        </button>
        <button
          id={`${tabIdPrefix}-tab-signup`}
          type="button"
          role="tab"
          aria-controls={`${tabIdPrefix}-panel-signup`}
          aria-selected={mode === "signup"}
          className={`tab ${mode === "signup" ? "tab-active" : ""}`}
          onClick={() => setMode("signup")}
        >
          Create account
        </button>
        <button
          id={`${tabIdPrefix}-tab-reset`}
          type="button"
          role="tab"
          aria-controls={`${tabIdPrefix}-panel-reset`}
          aria-selected={mode === "reset"}
          className={`tab ${mode === "reset" ? "tab-active" : ""}`}
          onClick={() => setMode("reset")}
        >
          Reset password
        </button>
      </div>

      {mode === "password" ? (
        <form id={`${tabIdPrefix}-panel-password`} role="tabpanel" aria-labelledby={`${tabIdPrefix}-tab-password`} className="space-y-2" onSubmit={handleSignIn}>
          <FormField
            type="email"
            label="Email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            disabled={!isConfigured || loading !== null}
            placeholder="trainer@example.com"
            autoComplete="email"
          />
          <FormField
            type="password"
            label="Password"
            required
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            disabled={!isConfigured || loading !== null}
            autoComplete="current-password"
          />
          <button
            type="submit"
            className="btn btn-sm btn-primary"
            disabled={!isConfigured || loading !== null}
          >
            {loading === "sign-in" ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Sign in
          </button>
        </form>
      ) : null}

      {mode === "magic" ? (
        <form id={`${tabIdPrefix}-panel-magic`} role="tabpanel" aria-labelledby={`${tabIdPrefix}-tab-magic`} className="space-y-2" onSubmit={handleMagicLink}>
          <FormField
            type="email"
            label="Email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            disabled={!isConfigured || loading !== null}
            placeholder="trainer@example.com"
            autoComplete="email"
          />
          <button
            type="submit"
            className="btn btn-sm btn-primary"
            disabled={!isConfigured || loading !== null}
          >
            {loading === "magic-link" ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Send magic link
          </button>
        </form>
      ) : null}

      {mode === "signup" ? (
        <form id={`${tabIdPrefix}-panel-signup`} role="tabpanel" aria-labelledby={`${tabIdPrefix}-tab-signup`} className="space-y-2" onSubmit={handleSignUp}>
          <FormField
            type="email"
            label="Email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            disabled={!isConfigured || loading !== null}
            placeholder="trainer@example.com"
            autoComplete="email"
          />
          <FormField
            type="password"
            label="Password"
            required
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            disabled={!isConfigured || loading !== null}
            autoComplete="new-password"
          />
          <FormField
            type="password"
            label="Confirm password"
            required
            value={confirmPassword}
            onChange={(event) => setConfirmPassword(event.target.value)}
            disabled={!isConfigured || loading !== null}
            autoComplete="new-password"
          />
          <button
            type="submit"
            className="btn btn-sm btn-primary"
            disabled={!isConfigured || loading !== null}
          >
            {loading === "sign-up" ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Create account
          </button>
        </form>
      ) : null}

      {mode === "reset" ? (
        <form id={`${tabIdPrefix}-panel-reset`} role="tabpanel" aria-labelledby={`${tabIdPrefix}-tab-reset`} className="space-y-2" onSubmit={handleResetPassword}>
          <FormField
            type="email"
            label="Email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            disabled={!isConfigured || loading !== null}
            placeholder="trainer@example.com"
            autoComplete="email"
          />
          <button
            type="submit"
            className="btn btn-sm btn-primary"
            disabled={!isConfigured || loading !== null}
          >
            {loading === "reset" ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Send reset email
          </button>
        </form>
      ) : null}

      {oauthProviders.length ? (
        <>
          <div className="divider" />
          <p className="text-xs text-base-content/70">Continue with</p>
          <div className="flex flex-col gap-2">
            {oauthProviders.map((provider) => (
              <button
                key={provider}
                type="button"
                className="btn btn-sm btn-outline"
                disabled={!isConfigured || loading !== null}
                onClick={() => handleOAuth(provider)}
              >
                {loading === "oauth" ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
                {toProviderLabel(provider)}
              </button>
            ))}
          </div>
        </>
      ) : null}
    </section>
  );
};

export default AuthPanel;
