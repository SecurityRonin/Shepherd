import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";

// Mock the api module — CloudSettings now uses typed API helpers, not raw fetch
vi.mock("../../../lib/api", () => ({
  getCloudStatus: vi.fn(),
  getLoginUrl: vi.fn(),
  getProfile: vi.fn(),
  getBalance: vi.fn(),
  logout: vi.fn(),
}));

import {
  getCloudStatus,
  getLoginUrl,
  getProfile,
  getBalance,
  logout as apiLogout,
} from "../../../lib/api";

const SAMPLE_PROFILE = {
  user_id: "u-1",
  email: "test@example.com",
  display_name: "testuser",
  plan: "pro",
  credits_balance: 42,
};

const SAMPLE_FREE_PROFILE = {
  user_id: "u-2",
  email: "free@example.com",
  display_name: "freeuser",
  plan: "free",
  credits_balance: 0,
};

const SAMPLE_BALANCE = {
  plan: "pro",
  credits_balance: 42,
  subscription_url: "https://example.com/subscribe",
  topup_url: "https://example.com/topup",
};

beforeEach(() => {
  vi.restoreAllMocks();
});

describe("CloudSettings", () => {
  it("renders loading state initially", async () => {
    // Never resolve so we stay in loading
    vi.mocked(getCloudStatus).mockReturnValue(new Promise(() => {}));
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    expect(screen.getByTestId("cloud-loading")).toBeInTheDocument();
    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("renders 'Sign in' when unauthenticated", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: false,
      plan: null,
      credits_balance: null,
      cloud_generation_enabled: false,
    });
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-unauthenticated")).toBeInTheDocument());
    expect(screen.getByTestId("sign-in-button")).toBeInTheDocument();
  });

  it("renders unavailable when cloud not configured", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: false,
      authenticated: false,
      plan: null,
      credits_balance: null,
      cloud_generation_enabled: false,
    });
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-unavailable")).toBeInTheDocument());
  });

  it("renders unavailable when getCloudStatus throws", async () => {
    vi.mocked(getCloudStatus).mockRejectedValue(new Error("Network error"));
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-unavailable")).toBeInTheDocument());
  });

  it("renders email and plan when authenticated", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockResolvedValue(SAMPLE_PROFILE);
    vi.mocked(getBalance).mockResolvedValue(SAMPLE_BALANCE);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-authenticated")).toBeInTheDocument());
    expect(screen.getByTestId("user-email")).toHaveTextContent("test@example.com");
    expect(screen.getByText("pro")).toBeInTheDocument();
  });

  it("renders authenticated free user with plan badge", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "free",
      credits_balance: 0,
      cloud_generation_enabled: false,
    });
    vi.mocked(getProfile).mockResolvedValue(SAMPLE_FREE_PROFILE);
    vi.mocked(getBalance).mockResolvedValue({ ...SAMPLE_BALANCE, plan: "free", credits_balance: 0 });
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-authenticated")).toBeInTheDocument());
    expect(screen.getByTestId("user-email")).toHaveTextContent("free@example.com");
    expect(screen.getByText("free")).toBeInTheDocument();
  });

  it("falls back to display_name when email is null", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockResolvedValue({
      ...SAMPLE_PROFILE,
      email: null,
      display_name: "fallback-name",
    });
    vi.mocked(getBalance).mockResolvedValue(SAMPLE_BALANCE);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-authenticated")).toBeInTheDocument());
    expect(screen.getByTestId("user-email")).toHaveTextContent("fallback-name");
  });

  it("renders credit balance", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockResolvedValue(SAMPLE_PROFILE);
    vi.mocked(getBalance).mockResolvedValue(SAMPLE_BALANCE);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("credit-balance")).toBeInTheDocument());
    expect(screen.getByTestId("credit-balance")).toHaveTextContent("42");
  });

  it("renders trial badges for all features", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockResolvedValue(SAMPLE_PROFILE);
    vi.mocked(getBalance).mockResolvedValue(SAMPLE_BALANCE);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("trial-logo")).toBeInTheDocument());
    expect(screen.getByTestId("trial-count-logo")).toHaveTextContent("2 remaining");
    // Verify all 7 features are present
    for (const feature of ["logo", "name", "northstar", "scrape", "crawl", "vision", "search"]) {
      expect(screen.getByTestId(`trial-${feature}`)).toBeInTheDocument();
    }
  });

  it("login button calls getLoginUrl and opens window", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: false,
      plan: null,
      credits_balance: null,
      cloud_generation_enabled: false,
    });
    vi.mocked(getLoginUrl).mockResolvedValue({ login_url: "https://auth.example.com/login" });
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("sign-in-button")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("sign-in-button"));
    await waitFor(() => expect(getLoginUrl).toHaveBeenCalledTimes(1));
    expect(openSpy).toHaveBeenCalledWith("https://auth.example.com/login", "_blank");
    openSpy.mockRestore();
  });

  it("sign out button calls logout API and returns to unauthenticated", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockResolvedValue(SAMPLE_PROFILE);
    vi.mocked(getBalance).mockResolvedValue(SAMPLE_BALANCE);
    vi.mocked(apiLogout).mockResolvedValue({ success: true });
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("logout-button")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("logout-button"));
    await waitFor(() => expect(apiLogout).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.getByTestId("cloud-unauthenticated")).toBeInTheDocument());
  });

  it("shows topup link from balance response", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockResolvedValue(SAMPLE_PROFILE);
    vi.mocked(getBalance).mockResolvedValue(SAMPLE_BALANCE);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("topup-link")).toBeInTheDocument());
    expect(screen.getByTestId("topup-link")).toHaveAttribute("href", "https://example.com/topup");
  });

  it("falls back to unauthenticated when profile fails", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockRejectedValue(new Error("Profile error"));
    vi.mocked(getBalance).mockResolvedValue(SAMPLE_BALANCE);
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-unauthenticated")).toBeInTheDocument());
  });

  it("renders authenticated even when balance fails", async () => {
    vi.mocked(getCloudStatus).mockResolvedValue({
      cloud_available: true,
      authenticated: true,
      plan: "pro",
      credits_balance: 42,
      cloud_generation_enabled: true,
    });
    vi.mocked(getProfile).mockResolvedValue(SAMPLE_PROFILE);
    vi.mocked(getBalance).mockRejectedValue(new Error("Balance error"));
    const { CloudSettings } = await import("../CloudSettings");
    render(<CloudSettings />);
    await waitFor(() => expect(screen.getByTestId("cloud-authenticated")).toBeInTheDocument());
    // topup link should fall back to "#"
    expect(screen.getByTestId("topup-link")).toHaveAttribute("href", "#");
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
    expect(screen.getByTestId("trial-count-name")).toHaveTextContent("2 remaining");
  });

  it("capitalizes the feature name", async () => {
    const { TrialBadge } = await import("../TrialBadge");
    render(<TrialBadge feature="northstar" remaining={5} />);
    const featureLabel = screen.getByText("northstar");
    expect(featureLabel.className).toContain("capitalize");
  });

  it("renders correct data-testid for feature", async () => {
    const { TrialBadge } = await import("../TrialBadge");
    render(<TrialBadge feature="vision" remaining={1} />);
    expect(screen.getByTestId("trial-vision")).toBeInTheDocument();
    expect(screen.getByTestId("trial-count-vision")).toHaveTextContent("1 remaining");
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

  it("displays zero balance", async () => {
    const { CreditBalance } = await import("../CreditBalance");
    render(<CreditBalance balance={0} topupUrl="/top-up" />);
    expect(screen.getByTestId("credit-balance")).toHaveTextContent("0");
  });

  it("shows 'credits available' label", async () => {
    const { CreditBalance } = await import("../CreditBalance");
    render(<CreditBalance balance={50} topupUrl="/top-up" />);
    expect(screen.getByText("credits available")).toBeInTheDocument();
  });

  it("topup link opens in new tab", async () => {
    const { CreditBalance } = await import("../CreditBalance");
    render(<CreditBalance balance={50} topupUrl="/top-up" />);
    const link = screen.getByTestId("topup-link");
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noreferrer");
  });
});
