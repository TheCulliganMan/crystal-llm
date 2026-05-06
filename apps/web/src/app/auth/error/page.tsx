import Link from "next/link";

const ERROR_LABELS: Record<string, string> = {
  oauth_cancelled: "Sign-in was cancelled before completion.",
  oauth_callback_failed: "The OAuth provider did not complete the callback.",
  auth_session_missing: "No active session was found for this request.",
  access_denied: "Access was denied by the identity provider.",
};

const resolveMessage = (error: string | undefined): string => {
  if (!error) {
    return "We couldn't complete sign-in. Please try again.";
  }
  return ERROR_LABELS[error] ?? "We couldn't complete sign-in. Please retry from the homepage.";
};

const AuthErrorPage = ({ searchParams }: { searchParams?: Record<string, string | string[] | undefined> }) => {
  const errorCode = typeof searchParams?.error === "string" ? searchParams.error : undefined;
  const message = resolveMessage(errorCode);

  return (
    <main className="mx-auto flex min-h-[70vh] w-full max-w-2xl items-center justify-center px-4 py-10">
      <section className="flex w-full flex-col gap-3">
        <div className="card border border-base-300 bg-base-100">
          <div className="card-body gap-1">
            <p className="text-xs uppercase tracking-[0.2em] text-base-content/60">Auth callback</p>
            <h1 className="text-3xl font-bold tracking-tight">Authentication error</h1>
          </div>
        </div>

        <div className="card border border-base-300 bg-base-200">
          <div className="card-body gap-2">
            <div role="alert" className="alert alert-error">
              <span className="text-sm">{message}</span>
            </div>
            <p className="text-sm text-base-content/70">
              If this keeps happening, clear cookies for this site and retry login.
            </p>
            <Link href="/" className="btn btn-outline btn-sm w-fit">
              Return home
            </Link>
          </div>
        </div>
      </section>
    </main>
  );
};

export default AuthErrorPage;
