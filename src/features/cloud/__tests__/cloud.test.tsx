import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";

const SAMPLE_PROFILE = {
  user_id: "u-1",
  email: "test@example.com",
  github_handle: "testuser",
  plan: "pro",
  credits_balance: 42,
  trial_counts: { logo: 2, name: 1, northstar: 0, scrape: 0, crawl: 1, vision: 2, search: 0 },
};

beforeEach(() => {
  vi.restoreAllMocks();
});

describe("CloudSettings", () => {
  it("renders 'Sign in' when unauthenticated (fetch returns 401)", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve({ ok: false, json: () => Promise.resolve(null) })));
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-unauthenticated")).toBeInTheDocument());
    expect(screen.getByTestId("sign-in-button")).toBeInTheDocument();
  });

  it("renders email and plan when authenticated", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve(SAMPLE_PROFILE),
    })));
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-authenticated")).toBeInTheDocument());
    expect(screen.getByTestId("user-email")).toHaveTextContent("test@example.com");
    expect(screen.getByText("pro")).toBeInTheDocument();
  });

  it("renders credit balance", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve(SAMPLE_PROFILE),
    })));
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("credit-balance")).toBeInTheDocument());
    expect(screen.getByTestId("credit-balance")).toHaveTextContent("42");
  });

  it("renders trial badges for all features", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve(SAMPLE_PROFILE),
    })));
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("trial-logo")).toBeInTheDocument());
    // logo: 2 used → 0 remaining
    expect(screen.getByTestId("trial-count-logo")).toHaveTextContent("0 remaining");
    // name: 1 used → 1 remaining
    expect(screen.getByTestId("trial-count-name")).toHaveTextContent("1 remaining");
  });

  it("sign out button clears profile", async () => {
    const mockFetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(SAMPLE_PROFILE) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(null) });
    vi.stubGlobal("fetch", mockFetch);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("logout-button")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("logout-button"));
    await waitFor(() => expect(screen.getByTestId("cloud-unauthenticated")).toBeInTheDocument());
  });
});

describe("TrialBadge", () => {
  it("shows 0 remaining in red", async () => {
    const { TrialBadge } = await import("../TrialBadge");
    render(<TrialBadge feature="logo" remaining={0} />);
    expect(screen.getByTestId("trial-count-logo").className).toContain("text-red-400");
    expect(screen.getByTestId("trial-count-logo")).toHaveTextContent("0 remaining");
  });

  it("shows non-zero remaining in green", async () => {
    const { TrialBadge } = await import("../TrialBadge");
    render(<TrialBadge feature="name" remaining={2} />);
    expect(screen.getByTestId("trial-count-name").className).toContain("text-green-400");
  });
});

describe("CreditBalance", () => {
  it("displays the balance number", async () => {
    const { CreditBalance } = await import("../CreditBalance");
    render(<CreditBalance balance={99} topupUrl="/api/credits/purchase" />);
    expect(screen.getByTestId("credit-balance")).toHaveTextContent("99");
  });

  it("includes a topup link", async () => {
    const { CreditBalance } = await import("../CreditBalance");
    render(<CreditBalance balance={10} topupUrl="/api/credits/purchase" />);
    expect(screen.getByTestId("topup-link")).toHaveAttribute("href", "/api/credits/purchase");
  });
});
