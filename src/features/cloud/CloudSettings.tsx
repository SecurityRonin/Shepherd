import React, { useEffect, useState, useCallback } from "react";
import { TrialBadge } from "./TrialBadge";
import { CreditBalance } from "./CreditBalance";
import { ErrorDisplay } from "../shared/ErrorDisplay";
import {
  getCloudStatus,
  getLoginUrl,
  getProfile,
  getBalance,
  logout as apiLogout,
  type CloudStatusResponse,
  type ProfileResponse,
  type CreditBalanceResponse,
} from "../../lib/api";

const TRIAL_LIMIT = 2;

type CloudState =
  | { phase: "loading" }
  | { phase: "unavailable" }
  | { phase: "unauthenticated" }
  | { phase: "authenticated"; profile: ProfileResponse; balance: CreditBalanceResponse | null };

export const CloudSettings: React.FC = () => {
  const [state, setState] = useState<CloudState>({ phase: "loading" });
  const [actionError, setActionError] = useState<string | null>(null);

  const loadCloudState = useCallback(async () => {
    try {
      const status: CloudStatusResponse = await getCloudStatus();
      if (!status.cloud_available) {
        setState({ phase: "unavailable" });
        return;
      }
      if (!status.authenticated) {
        setState({ phase: "unauthenticated" });
        return;
      }
      // Authenticated — load profile and balance
      const [profile, balance] = await Promise.allSettled([getProfile(), getBalance()]);
      const prof = profile.status === "fulfilled" ? profile.value : null;
      const bal = balance.status === "fulfilled" ? balance.value : null;
      if (prof) {
        setState({ phase: "authenticated", profile: prof, balance: bal });
      } else {
        setState({ phase: "unauthenticated" });
      }
    } catch {
      setState({ phase: "unavailable" });
    }
  }, []);

  useEffect(() => {
    loadCloudState();
  }, [loadCloudState]);

  const handleLogin = async () => {
    setActionError(null);
    try {
      const { login_url } = await getLoginUrl();
      window.open(login_url, "_blank");
    } catch (err) {
      setActionError(err instanceof Error ? err.message : "Login failed");
    }
  };

  const handleLogout = async () => {
    setActionError(null);
    try {
      await apiLogout();
      setState({ phase: "unauthenticated" });
    } catch (err) {
      setActionError(err instanceof Error ? err.message : "Logout failed");
    }
  };

  if (state.phase === "loading") {
    return <div className="p-6 text-gray-400" data-testid="cloud-loading">Loading...</div>;
  }

  if (state.phase === "unavailable") {
    return (
      <div className="p-6 text-center text-gray-400" data-testid="cloud-unavailable">
        <h2 className="text-xl font-semibold text-white mb-2">Cloud Features</h2>
        <p className="text-sm">Cloud features are not configured. Set up your shepherd.pro account to enable AI-powered features.</p>
      </div>
    );
  }

  if (state.phase === "unauthenticated") {
    return (
      <div className="p-6 flex flex-col items-center gap-4" data-testid="cloud-unauthenticated">
        <h2 className="text-xl font-semibold text-white">Cloud Features</h2>
        <p className="text-gray-400 text-sm text-center max-w-sm">
          Sign in to access AI-powered logo generation, name suggestions, NorthStar advisor, and more.
        </p>
        <button
          onClick={handleLogin}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
          data-testid="sign-in-button"
        >
          Sign in with GitHub
        </button>
        <ErrorDisplay message={actionError} testId="cloud-error" />
      </div>
    );
  }

  const { profile, balance } = state;
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

      <ErrorDisplay message={actionError} testId="cloud-error" />

      <div className="text-sm text-gray-300">
        <span data-testid="user-email">{profile.email ?? profile.display_name ?? "Unknown user"}</span>
        <span className="ml-2 px-2 py-0.5 rounded text-xs bg-gray-700 text-gray-300 capitalize">
          {profile.plan}
        </span>
      </div>

      <CreditBalance
        balance={profile.credits_balance}
        topupUrl={balance?.topup_url ?? "#"}
      />

      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-2">Features</h3>
        {features.map((f) => (
          <TrialBadge
            key={f}
            feature={f}
            remaining={TRIAL_LIMIT}
          />
        ))}
      </div>
    </div>
  );
};
