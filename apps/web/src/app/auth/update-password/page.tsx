"use client";

import { Suspense, useMemo, useState, type FormEvent } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useSupabase } from "@/components/providers/supabase-provider";
import { sanitizeNextPath } from "@/lib/supabase/urls";

type StatusMessage = {
  text: string;
  severity: "success" | "info" | "warning" | "error";
};

const UpdatePasswordForm = () => {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { supabaseClient, isConfigured } = useSupabase();
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [message, setMessage] = useState<StatusMessage | null>(null);
  const [loading, setLoading] = useState(false);

  const redirectTo = useMemo(() => {
    const next = searchParams.get("next");
    return sanitizeNextPath(next) ?? "/";
  }, [searchParams]);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!supabaseClient) {
      setMessage({ text: "Account services are currently unavailable.", severity: "warning" });
      return;
    }
    if (!password || !confirmPassword) {
      setMessage({ text: "Enter and confirm your new password.", severity: "warning" });
      return;
    }
    if (password !== confirmPassword) {
      setMessage({ text: "Passwords do not match.", severity: "error" });
      return;
    }
    setLoading(true);
    setMessage(null);
    const { error } = await supabaseClient.auth.updateUser({ password });
    if (error) {
      setMessage({ text: error.message, severity: "error" });
    } else {
      setMessage({ text: "Password updated successfully.", severity: "success" });
      router.push(redirectTo);
    }
    setLoading(false);
  };

  return (
    <main className="mx-auto w-full max-w-6xl px-4">
      <section className="w-full space-y-4 pt-2 pb-4 md:pt-3">
        <article className="card card-compact card-bordered bg-base-200">
          <div className="card-body gap-1">
            <p className="text-xs uppercase tracking-wide text-base-content/70">Account</p>
            <h1 className="text-2xl font-semibold">Update password</h1>
            <p className="text-sm text-base-content/70">
              Set a new password for your account.
            </p>
          </div>
        </article>

        <article className="card card-bordered bg-base-200">
          <div className="card-body gap-2">
            {!isConfigured ? (
              <div role="alert" className="alert alert-warning">
                <span>Account services are currently unavailable. Please try again shortly.</span>
              </div>
            ) : null}

            {message ? (
              <div
                role="alert"
                className={`alert ${message.severity === "error" ? "alert-error" : message.severity === "warning" ? "alert-warning" : "alert-info"}`}
              >
                <span>{message.text}</span>
              </div>
            ) : null}

            <form className="space-y-3" onSubmit={handleSubmit}>
              <label className="form-control w-full">
                <span className="label">
                  <span className="label-text text-sm">New password</span>
                </span>
                <input
                  type="password"
                  className="input input-bordered w-full"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  disabled={!isConfigured || loading}
                  autoComplete="new-password"
                />
              </label>
              <label className="form-control w-full">
                <span className="label">
                  <span className="label-text text-sm">Confirm password</span>
                </span>
                <input
                  type="password"
                  className="input input-bordered w-full"
                  value={confirmPassword}
                  onChange={(event) => setConfirmPassword(event.target.value)}
                  disabled={!isConfigured || loading}
                  autoComplete="new-password"
                />
              </label>
              <button
                type="submit"
                className="btn btn-primary"
                disabled={!isConfigured || loading}
              >
                Update password
              </button>
            </form>
          </div>
        </article>
      </section>
    </main>
  );
};

const UpdatePasswordPage = () => (
  <Suspense fallback={null}>
    <UpdatePasswordForm />
  </Suspense>
);

export default UpdatePasswordPage;
