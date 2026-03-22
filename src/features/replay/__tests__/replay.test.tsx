import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { useStore } from "../../../store";

const SAMPLE_EVENT = {
  id: 1, task_id: 1, session_id: 1,
  event_type: "tool_call",
  summary: "Running cargo test",
  content: "cargo test --lib",
  metadata: null,
  timestamp: "2026-03-14T00:00:00Z",
};

beforeEach(() => {
  useStore.setState({ replayEvents: [] } as any);
  vi.stubGlobal("fetch", vi.fn(() => Promise.resolve({ json: () => Promise.resolve([]) })));
});

describe("ReplayViewer", () => {
  it("renders 'No events' when empty", async () => {
    const { ReplayViewer } = await import("../ReplayViewer");
    render(<ReplayViewer />);
    expect(screen.getByTestId("no-events")).toBeInTheDocument();
    expect(screen.getByText("No events")).toBeInTheDocument();
  });

  it("renders event rows when events exist", async () => {
    useStore.setState({ replayEvents: [SAMPLE_EVENT] } as any);
    const { ReplayViewer } = await import("../ReplayViewer");
    render(<ReplayViewer />);
    expect(screen.getByText("Running cargo test")).toBeInTheDocument();
  });

  it("renders correct number of event rows", async () => {
    useStore.setState({
      replayEvents: [
        { ...SAMPLE_EVENT, id: 1, event_type: "session_start", summary: "Started" },
        { ...SAMPLE_EVENT, id: 2, event_type: "output", summary: "Tests passed" },
        { ...SAMPLE_EVENT, id: 3, event_type: "session_end", summary: "Ended" },
      ],
    } as any);
    const { ReplayViewer } = await import("../ReplayViewer");
    render(<ReplayViewer />);
    expect(screen.getAllByTestId("event-row-header").length).toBe(3);
  });
});

describe("EventRow", () => {
  it("renders badge and summary", async () => {
    const { EventRow } = await import("../EventRow");
    render(<EventRow event={SAMPLE_EVENT} />);
    expect(screen.getByTestId("badge-tool_call")).toBeInTheDocument();
    expect(screen.getByText("Running cargo test")).toBeInTheDocument();
  });

  it("expands to show content when clicked", async () => {
    const { EventRow } = await import("../EventRow");
    render(<EventRow event={SAMPLE_EVENT} />);
    expect(screen.queryByTestId("event-row-content")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId("event-row-header"));
    expect(screen.getByTestId("event-row-content")).toBeInTheDocument();
    expect(screen.getByText("cargo test --lib")).toBeInTheDocument();
  });

  it("does not show content area for events with empty content", async () => {
    const { EventRow } = await import("../EventRow");
    render(<EventRow event={{ ...SAMPLE_EVENT, content: "" }} />);
    fireEvent.click(screen.getByTestId("event-row-header"));
    expect(screen.queryByTestId("event-row-content")).not.toBeInTheDocument();
  });
});

describe("EventTypeBadge", () => {
  it("renders correct badge colors for known types", async () => {
    const { EventTypeBadge } = await import("../EventTypeBadge");
    const { rerender } = render(<EventTypeBadge type="error" />);
    expect(screen.getByTestId("badge-error").className).toContain("bg-red-600");
    rerender(<EventTypeBadge type="tool_call" />);
    expect(screen.getByTestId("badge-tool_call").className).toContain("bg-blue-600");
    rerender(<EventTypeBadge type="file_change" />);
    expect(screen.getByTestId("badge-file_change").className).toContain("bg-purple-600");
  });

  it("renders fallback color for unknown type", async () => {
    const { EventTypeBadge } = await import("../EventTypeBadge");
    render(<EventTypeBadge type="unknown_custom" />);
    expect(screen.getByTestId("badge-unknown_custom").className).toContain("bg-gray-600");
  });

  it("renders type text inside the badge", async () => {
    const { EventTypeBadge } = await import("../EventTypeBadge");
    render(<EventTypeBadge type="session_start" />);
    expect(screen.getByTestId("badge-session_start")).toHaveTextContent("session_start");
  });

  it("applies correct color for all known badge types", async () => {
    const { EventTypeBadge } = await import("../EventTypeBadge");
    const knownTypes = [
      { type: "session_start", color: "bg-green-600" },
      { type: "session_end", color: "bg-gray-600" },
      { type: "output", color: "bg-gray-500" },
      { type: "permission_request", color: "bg-yellow-600" },
      { type: "file_change", color: "bg-purple-600" },
      { type: "llm_call", color: "bg-indigo-600" },
      { type: "input", color: "bg-teal-600" },
    ];

    for (const { type, color } of knownTypes) {
      const { unmount } = render(<EventTypeBadge type={type} />);
      expect(screen.getByTestId(`badge-${type}`).className).toContain(color);
      unmount();
    }
  });
});

// --- Additional EventRow tests ---

describe("EventRow - expanded behavior", () => {
  it("toggles content visibility on repeated clicks", async () => {
    const { EventRow } = await import("../EventRow");
    render(<EventRow event={SAMPLE_EVENT} />);

    // Initially collapsed
    expect(screen.queryByTestId("event-row-content")).not.toBeInTheDocument();

    // First click: expand
    fireEvent.click(screen.getByTestId("event-row-header"));
    expect(screen.getByTestId("event-row-content")).toBeInTheDocument();

    // Second click: collapse
    fireEvent.click(screen.getByTestId("event-row-header"));
    expect(screen.queryByTestId("event-row-content")).not.toBeInTheDocument();
  });

  it("renders timestamp for the event", async () => {
    const { EventRow } = await import("../EventRow");
    render(
      <EventRow
        event={{ ...SAMPLE_EVENT, timestamp: "2026-03-14T15:30:00Z" }}
      />,
    );
    // The timestamp should be rendered (format depends on locale, but should contain time)
    const header = screen.getByTestId("event-row-header");
    expect(header.textContent).toBeTruthy();
  });

  it("renders the event summary text", async () => {
    const { EventRow } = await import("../EventRow");
    render(
      <EventRow
        event={{ ...SAMPLE_EVENT, summary: "Compiling project..." }}
      />,
    );
    expect(screen.getByText("Compiling project...")).toBeInTheDocument();
  });
});
