import React, { useEffect, useRef } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { useStore } from "../../store";
import "@xterm/xterm/css/xterm.css";

interface TerminalProps {
  taskId: number;
}

export const Terminal: React.FC<TerminalProps> = ({ taskId }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  const wsClient = useStore((s) => s.wsClient);
  const registerTerminalHandler = useStore((s) => s.registerTerminalHandler);
  const unregisterTerminalHandler = useStore((s) => s.unregisterTerminalHandler);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new XTerm({
      theme: {
        background: "#09090b",
        foreground: "#e4e4e7",
        cursor: "#e4e4e7",
        selectionBackground: "#27272a",
        black: "#09090b",
        red: "#ef4444",
        green: "#22c55e",
        yellow: "#eab308",
        blue: "#3b82f6",
        magenta: "#a855f7",
        cyan: "#06b6d4",
        white: "#e4e4e7",
        brightBlack: "#52525b",
        brightRed: "#f87171",
        brightGreen: "#4ade80",
        brightYellow: "#facc15",
        brightBlue: "#60a5fa",
        brightMagenta: "#c084fc",
        brightCyan: "#22d3ee",
        brightWhite: "#fafafa",
      },
      fontFamily: "ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo, monospace",
      fontSize: 13,
      lineHeight: 1.3,
      cursorBlink: true,
      scrollback: 10000,
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    term.open(containerRef.current);

    // Initial fit after DOM layout
    requestAnimationFrame(() => {
      try {
        fitAddon.fit();
      } catch {
        // Container may not be visible yet
      }
    });

    terminalRef.current = term;
    fitAddonRef.current = fitAddon;

    // Register a handler so server terminal_output events get written to this terminal
    registerTerminalHandler(taskId, (data: string) => {
      term.write(data);
    });

    // --- WebSocket I/O (input & resize) ---
    let dataDisposable: { dispose: () => void } | undefined;
    let resizeDisposable: { dispose: () => void } | undefined;

    if (wsClient) {
      // Forward keystrokes to the backend PTY
      dataDisposable = term.onData((data) => {
        wsClient.send({
          type: "terminal_input",
          data: { task_id: taskId, data },
        });
      });

      // Notify backend when the terminal is resized
      resizeDisposable = term.onResize(({ cols, rows }) => {
        wsClient.send({
          type: "terminal_resize",
          data: { task_id: taskId, cols, rows },
        });
      });

      // Send initial size so the backend PTY is sized correctly from the start
      requestAnimationFrame(() => {
        try {
          fitAddon.fit();
          wsClient.send({
            type: "terminal_resize",
            data: { task_id: taskId, cols: term.cols, rows: term.rows },
          });
        } catch {
          // Container may not be ready
        }
      });
    }

    // ResizeObserver for auto-fit
    const resizeObserver = new ResizeObserver(() => {
      try {
        fitAddon.fit();
      } catch {
        // Ignore errors when container is hidden or detached
      }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      unregisterTerminalHandler(taskId);
      dataDisposable?.dispose();
      resizeDisposable?.dispose();
      resizeObserver.disconnect();
      term.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
    };
  }, [taskId, wsClient, registerTerminalHandler, unregisterTerminalHandler]);

  return (
    <div className="flex-1 flex flex-col min-h-0">
      {/* Terminal toolbar */}
      <div className="flex items-center px-3 py-1.5 bg-shepherd-surface border-b border-shepherd-border">
        <span className="text-[11px] font-medium text-shepherd-muted uppercase tracking-wider">
          Terminal
        </span>
        <span className="ml-2 text-[10px] text-shepherd-muted font-mono">
          Task #{taskId}
        </span>
      </div>
      {/* Terminal container */}
      <div
        ref={containerRef}
        className="flex-1 min-h-0 bg-[#09090b] p-1"
        data-testid="terminal-container"
      />
    </div>
  );
};
