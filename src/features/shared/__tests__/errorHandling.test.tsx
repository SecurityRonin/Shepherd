import { describe, it, expect, vi, beforeAll, afterAll } from "vitest";
import { render, screen } from "@testing-library/react";
import { renderHook, act } from "@testing-library/react";

describe("ErrorDisplay", () => {
  it("renders nothing when message is null", async () => {
    const { ErrorDisplay } = await import("../ErrorDisplay");
    const { container } = render(<ErrorDisplay message={null} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders error message with data-testid", async () => {
    const { ErrorDisplay } = await import("../ErrorDisplay");
    render(<ErrorDisplay message="Something broke" />);
    expect(screen.getByTestId("error-display")).toBeInTheDocument();
    expect(screen.getByText("Something broke")).toBeInTheDocument();
  });

  it("applies custom testId", async () => {
    const { ErrorDisplay } = await import("../ErrorDisplay");
    render(<ErrorDisplay message="Oops" testId="custom-error" />);
    expect(screen.getByTestId("custom-error")).toBeInTheDocument();
  });

  it("applies dark variant styling", async () => {
    const { ErrorDisplay } = await import("../ErrorDisplay");
    render(<ErrorDisplay message="Fail" variant="dark" />);
    expect(screen.getByTestId("error-display").className).toContain("bg-red-900");
  });

  it("applies light variant styling by default", async () => {
    const { ErrorDisplay } = await import("../ErrorDisplay");
    render(<ErrorDisplay message="Fail" />);
    expect(screen.getByTestId("error-display").className).toContain("bg-red-50");
  });
});

describe("LazyFallback", () => {
  it("renders loading skeleton with default label", async () => {
    const { LazyFallback } = await import("../LazyFallback");
    render(<LazyFallback />);
    expect(screen.getByTestId("lazy-fallback")).toBeInTheDocument();
    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("renders custom label", async () => {
    const { LazyFallback } = await import("../LazyFallback");
    render(<LazyFallback label="Loading editor..." />);
    expect(screen.getByText("Loading editor...")).toBeInTheDocument();
  });

  it("renders with custom testId", async () => {
    const { LazyFallback } = await import("../LazyFallback");
    render(<LazyFallback testId="terminal-loading" />);
    expect(screen.getByTestId("terminal-loading")).toBeInTheDocument();
  });

  it("renders animated pulse element", async () => {
    const { LazyFallback } = await import("../LazyFallback");
    render(<LazyFallback />);
    const fallback = screen.getByTestId("lazy-fallback");
    expect(fallback.className).toContain("animate-pulse");
  });
});

describe("ErrorBoundary", () => {
  // Suppress React error boundary console output
  const originalError = console.error;
  beforeAll(() => { console.error = vi.fn(); });
  afterAll(() => { console.error = originalError; });

  function ThrowingChild({ shouldThrow }: { shouldThrow: boolean }) {
    if (shouldThrow) throw new Error("Test crash");
    return <div data-testid="child">OK</div>;
  }

  it("renders children when no error", async () => {
    const { ErrorBoundary } = await import("../ErrorBoundary");
    render(
      <ErrorBoundary>
        <ThrowingChild shouldThrow={false} />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId("child")).toBeInTheDocument();
  });

  it("renders fallback UI when child throws", async () => {
    const { ErrorBoundary } = await import("../ErrorBoundary");
    render(
      <ErrorBoundary>
        <ThrowingChild shouldThrow={true} />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId("error-boundary-fallback")).toBeInTheDocument();
    expect(screen.getByText("Test crash")).toBeInTheDocument();
  });

  it("shows 'Reload' button in fallback", async () => {
    const { ErrorBoundary } = await import("../ErrorBoundary");
    render(
      <ErrorBoundary>
        <ThrowingChild shouldThrow={true} />
      </ErrorBoundary>,
    );
    expect(screen.getByRole("button", { name: /reload/i })).toBeInTheDocument();
  });
});

describe("useAsyncAction", () => {
  it("starts with idle state", async () => {
    const { useAsyncAction } = await import("../../../hooks/useAsyncAction");
    const { result } = renderHook(() => useAsyncAction(async () => "ok"));
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.data).toBeNull();
  });

  it("sets loading true during execution", async () => {
    let resolve: (v: string) => void;
    const promise = new Promise<string>((r) => { resolve = r; });
    const { useAsyncAction } = await import("../../../hooks/useAsyncAction");
    const { result } = renderHook(() => useAsyncAction(async () => promise));

    act(() => { result.current.execute(); });
    expect(result.current.loading).toBe(true);

    await act(async () => { resolve!("done"); });
    expect(result.current.loading).toBe(false);
    expect(result.current.data).toBe("done");
  });

  it("captures error message on failure", async () => {
    const { useAsyncAction } = await import("../../../hooks/useAsyncAction");
    const { result } = renderHook(() =>
      useAsyncAction(async () => { throw new Error("Failed"); }),
    );

    await act(async () => { result.current.execute(); });
    expect(result.current.error).toBe("Failed");
    expect(result.current.loading).toBe(false);
    expect(result.current.data).toBeNull();
  });

  it("resets error on new execution", async () => {
    let shouldFail = true;
    const { useAsyncAction } = await import("../../../hooks/useAsyncAction");
    const { result } = renderHook(() =>
      useAsyncAction(async () => {
        if (shouldFail) throw new Error("Fail");
        return "ok";
      }),
    );

    await act(async () => { result.current.execute(); });
    expect(result.current.error).toBe("Fail");

    shouldFail = false;
    await act(async () => { result.current.execute(); });
    expect(result.current.error).toBeNull();
    expect(result.current.data).toBe("ok");
  });

  it("passes arguments through to the action", async () => {
    const spy = vi.fn().mockResolvedValue("result");
    const { useAsyncAction } = await import("../../../hooks/useAsyncAction");
    const { result } = renderHook(() => useAsyncAction(spy));

    await act(async () => { result.current.execute("arg1", "arg2"); });
    expect(spy).toHaveBeenCalledWith("arg1", "arg2");
  });

  it("reset clears data and error", async () => {
    const { useAsyncAction } = await import("../../../hooks/useAsyncAction");
    const { result } = renderHook(() =>
      useAsyncAction(async () => "data"),
    );

    await act(async () => { result.current.execute(); });
    expect(result.current.data).toBe("data");

    act(() => { result.current.reset(); });
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });
});
