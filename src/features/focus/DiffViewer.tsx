import React, { useState } from "react";
import { DiffEditor } from "@monaco-editor/react";
import { useStore } from "../../store";

interface DiffViewerProps {
  taskId: number;
}

/** Map file extensions to Monaco language identifiers. */
export function detectLanguage(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    json: "json",
    md: "markdown",
    mdx: "markdown",
    html: "html",
    htm: "html",
    css: "css",
    scss: "scss",
    less: "less",
    py: "python",
    rs: "rust",
    go: "go",
    java: "java",
    kt: "kotlin",
    kts: "kotlin",
    rb: "ruby",
    php: "php",
    c: "c",
    h: "c",
    cpp: "cpp",
    hpp: "cpp",
    cc: "cpp",
    cs: "csharp",
    swift: "swift",
    sql: "sql",
    sh: "shell",
    bash: "shell",
    zsh: "shell",
    yaml: "yaml",
    yml: "yaml",
    toml: "ini",
    ini: "ini",
    xml: "xml",
    svg: "xml",
    dockerfile: "dockerfile",
    graphql: "graphql",
    gql: "graphql",
    lua: "lua",
    r: "r",
    dart: "dart",
    vue: "html",
    svelte: "html",
  };
  return map[ext] ?? "plaintext";
}

export const DiffViewer: React.FC<DiffViewerProps> = ({ taskId }) => {
  const task = useStore((s) => s.tasks[taskId]);
  const diffs = task?.diffs;

  const [activeFileIndex, setActiveFileIndex] = useState(0);
  const [renderSideBySide, setRenderSideBySide] = useState(false);

  // Empty state
  if (!diffs || diffs.length === 0) {
    return (
      <div className="flex-1 flex flex-col min-h-0">
        <div className="flex items-center px-3 py-1.5 bg-shepherd-surface border-b border-shepherd-border">
          <span className="text-[11px] font-medium text-shepherd-muted uppercase tracking-wider">
            Changes
          </span>
        </div>
        <div className="flex-1 flex items-center justify-center text-shepherd-muted text-sm">
          No file changes yet
        </div>
      </div>
    );
  }

  const activeDiff = diffs[activeFileIndex] ?? diffs[0];
  const language = activeDiff.language || detectLanguage(activeDiff.file_path);

  return (
    <div className="flex-1 flex flex-col min-h-0">
      {/* Toolbar: file tabs + view toggle */}
      <div className="flex items-center bg-shepherd-surface border-b border-shepherd-border overflow-hidden">
        {/* File tabs */}
        <div className="flex-1 flex items-center overflow-x-auto min-w-0">
          {diffs.map((diff, idx) => {
            const fileName = diff.file_path.split("/").pop() ?? diff.file_path;
            const isActive = idx === activeFileIndex;
            return (
              <button
                key={diff.file_path}
                onClick={() => setActiveFileIndex(idx)}
                className={`
                  px-3 py-1.5 text-[11px] font-mono whitespace-nowrap border-b-2 transition-colors
                  ${isActive
                    ? "text-shepherd-text border-shepherd-accent bg-shepherd-bg"
                    : "text-shepherd-muted border-transparent hover:text-shepherd-text hover:bg-shepherd-bg/50"
                  }
                `}
                title={diff.file_path}
              >
                {fileName}
              </button>
            );
          })}
        </div>

        {/* Unified / Side-by-Side toggle */}
        <div className="flex items-center gap-1 px-2 flex-shrink-0">
          <button
            onClick={() => setRenderSideBySide(false)}
            className={`px-2 py-1 text-[10px] rounded transition-colors ${
              !renderSideBySide
                ? "bg-shepherd-accent text-white"
                : "text-shepherd-muted hover:text-shepherd-text"
            }`}
          >
            Unified
          </button>
          <button
            onClick={() => setRenderSideBySide(true)}
            className={`px-2 py-1 text-[10px] rounded transition-colors ${
              renderSideBySide
                ? "bg-shepherd-accent text-white"
                : "text-shepherd-muted hover:text-shepherd-text"
            }`}
          >
            Split
          </button>
        </div>
      </div>

      {/* Diff editor */}
      <div className="flex-1 min-h-0">
        <DiffEditor
          original={activeDiff.before_content}
          modified={activeDiff.after_content}
          language={language}
          theme="shepherd-dark"
          options={{
            readOnly: true,
            renderSideBySide,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            fontSize: 12,
            lineHeight: 18,
            fontFamily: "ui-monospace, 'Cascadia Code', 'Source Code Pro', Menlo, monospace",
            glyphMargin: true,
            folding: true,
            lineNumbers: "on",
            renderOverviewRuler: false,
            overviewRulerLanes: 0,
          }}
          beforeMount={(monaco) => {
            // Define custom dark theme
            monaco.editor.defineTheme("shepherd-dark", {
              base: "vs-dark",
              inherit: true,
              rules: [],
              colors: {
                "editor.background": "#09090b",
                "editor.foreground": "#e4e4e7",
                "editorLineNumber.foreground": "#52525b",
                "editorLineNumber.activeForeground": "#a1a1aa",
                "editor.selectionBackground": "#27272a",
                "editor.lineHighlightBackground": "#18181b",
                "editorGutter.background": "#09090b",
                "diffEditor.insertedTextBackground": "#22c55e18",
                "diffEditor.removedTextBackground": "#ef444418",
                "diffEditor.insertedLineBackground": "#22c55e0d",
                "diffEditor.removedLineBackground": "#ef44440d",
              },
            });
          }}
        />
      </div>

      {/* File path breadcrumb */}
      <div className="px-3 py-1 bg-shepherd-surface border-t border-shepherd-border">
        <span className="text-[10px] text-shepherd-muted font-mono">
          {activeDiff.file_path}
        </span>
      </div>
    </div>
  );
};
