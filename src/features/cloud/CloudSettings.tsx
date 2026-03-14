import React, { useEffect, useState } from "react";
import { TrialBadge } from "./TrialBadge";
import { CreditBalance } from "./CreditBalance";

interface CachedProfile {
  user_id: string;
  email?: string;
  github_handle?: string;
  plan: "free" | "pro";
  credits_balance: number;
  trial_counts: {
    logo: number; name: number; northstar: number;
    scrape: number; crawl: number; vision: number; search: number;
  };
}

const TRIAL_LIMIT = 2;

export const CloudSettings: React.FC = () => {
  const [profile, setProfile] = useState<CachedProfile | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("/api/cloud/profile")
      .then((r) => {
        if (!r.ok) throw new Error("Not authenticated");
        return r.json();
      })
      .then(setProfile)
      .catch(() => setProfile(null))
      .finally(() => setLoading(false));
  }, []);

  const handleLogout = () => {
    fetch("/api/cloud/logout", { method: "POST" })
      .then(() => setProfile(null))
      .catch(() => {});
  };

  if (loading) {
    return <div className="p-6 text-gray-400" data-testid="cloud-loading">Loading...</div>;
  }

  if (!profile) {
    return (
      <div className="p-6 flex flex-col items-center gap-4" data-testid="cloud-unauthenticated">
        <h2 className="text-xl font-semibold text-white">Cloud Features</h2>
        <p className="text-gray-400 text-sm text-center max-w-sm">
          Sign in to access AI-powered logo generation, name suggestions, NorthStar advisor, and more.
        </p>
        <a
          href="/api/auth/login?provider=github"
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
          data-testid="sign-in-button"
        >
          Sign in with GitHub
        </a>
      </div>
    );
  }

  const features = ["logo", "name", "northstar", "scrape", "crawl", "vision", "search"] as const;

  return (
    <div className="p-6 space-y-6" data-testid="cloud-authenticated">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-white">Cloud Settings</h2>
        <button
          onClick={handleLogout}
          className="text-xs text-gray-400 hover:text-white"
          data-testid="logout-button"
        >
          Sign out
        </button>
      </div>

      <div className="text-sm text-gray-300">
        <span data-testid="user-email">{profile.email ?? profile.github_handle ?? "Unknown user"}</span>
        <span className="ml-2 px-2 py-0.5 rounded text-xs bg-gray-700 text-gray-300 capitalize">
          {profile.plan}
        </span>
      </div>

      <CreditBalance
        balance={profile.credits_balance}
        topupUrl="/api/credits/purchase"
      />

      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-2">Free Trials Remaining</h3>
        {features.map((f) => (
          <TrialBadge
            key={f}
            feature={f}
            remaining={Math.max(0, TRIAL_LIMIT - profile.trial_counts[f])}
          />
        ))}
      </div>
    </div>
  );
};
