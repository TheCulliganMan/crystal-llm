"use client";

import { useTransition, useState, FormEvent } from "react";
import { upsertProfileAction } from "@/arena/actions";
import type { ArenaProfile } from "@/arena/types";

export const ProfileForm = ({
  profile,
  userEmail,
}: {
  profile: ArenaProfile | null;
  userEmail?: string | null;
}) => {
  const [handle, setHandle] = useState(profile?.handle ?? "");
  const [displayName, setDisplayName] = useState(profile?.display_name ?? "");
  const [bio, setBio] = useState(profile?.bio ?? "");
  const [pending, startTransition] = useTransition();
  const [message, setMessage] = useState<string | null>(null);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setMessage(null);
    startTransition(async () => {
      await upsertProfileAction({
        handle,
        displayName,
        bio,
      });
      setMessage("Profile saved.");
    });
  };

  return (
    <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90">
      <div className="card-body space-y-4">
        <header className="flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <h2 className="card-title">Arena profile</h2>
            <p className="text-sm text-base-content/70">
              Sets your public handle and display name.
            </p>
          </div>
          {userEmail ? <span className="text-xs text-base-content/60">{userEmail}</span> : null}
        </header>
        <form onSubmit={submit} className="space-y-3">
          <label className="form-control">
            <span className="label label-text">Handle</span>
            <input
              required
              className="input input-bordered w-full"
              value={handle}
              onChange={(e) => setHandle(e.target.value)}
              placeholder="odinn_johto"
            />
          </label>
          <label className="form-control">
            <span className="label label-text">Display name</span>
            <input
              className="input input-bordered w-full"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="Violet City Runner"
            />
          </label>
          <label className="form-control">
            <span className="label label-text">Bio</span>
            <textarea
              className="textarea textarea-bordered h-24 w-full"
              value={bio}
              onChange={(e) => setBio(e.target.value)}
              placeholder="Route 29 optimization, badge rush builds, MCP botcraft."
            />
          </label>
          {message ? (
            <div role="alert" className="alert alert-success">
              <span>{message}</span>
            </div>
          ) : null}
          <button type="submit" disabled={pending} className="btn btn-primary">
            {pending ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
            Save profile
          </button>
        </form>
      </div>
    </section>
  );
};
