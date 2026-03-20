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
  });

  it("sign out button returns to unauthenticated", async () => {
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
