import React, { useState, useCallback } from "react";
import { generateLogo, exportLogo } from "../../lib/api";
import type { LogoVariant, ExportedFile } from "../../lib/api";
import { ErrorDisplay } from "../shared/ErrorDisplay";

// ── Constants ────────────────────────────────────────────────────────

const STYLES = [
  { id: "minimal", label: "Minimal", desc: "Clean, flat, modern" },
  { id: "geometric", label: "Geometric", desc: "Precise shapes, symmetry" },
  { id: "mascot", label: "Mascot", desc: "Friendly character, personality" },
  { id: "abstract", label: "Abstract", desc: "Creative, fluid forms" },
] as const;

type StyleId = (typeof STYLES)[number]["id"];

// ── Component ────────────────────────────────────────────────────────

export const LogoGenerator: React.FC = () => {
  const [productName, setProductName] = useState("");
  const [description, setDescription] = useState("");
  const [selectedStyle, setSelectedStyle] = useState<StyleId>("minimal");
  const [colors, setColors] = useState<[string, string]>(["#3B82F6", "#1E293B"]);
  const [variants, setVariants] = useState<LogoVariant[]>([]);
  const [selectedVariant, setSelectedVariant] = useState<number | null>(null);
  const [exportedFiles, setExportedFiles] = useState<ExportedFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleColorChange = useCallback(
    (index: 0 | 1, value: string) => {
      setColors((prev) => {
        const next: [string, string] = [...prev];
        next[index] = value;
        return next;
      });
    },
    [],
  );

  const generate = useCallback(async () => {
    if (!productName.trim()) return;

    setLoading(true);
    setError(null);
    setVariants([]);
    setSelectedVariant(null);
    setExportedFiles([]);

    try {
      const data = await generateLogo({
        product_name: productName.trim(),
        product_description: description.trim() || undefined,
        style: selectedStyle,
        colors: [...colors],
      });
      setVariants(data.variants);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Generation failed");
    } finally {
      setLoading(false);
    }
  }, [productName, description, selectedStyle, colors]);

  const doExport = useCallback(async () => {
    if (selectedVariant === null) return;

    const variant = variants.find((v) => v.index === selectedVariant);
    if (!variant) return;

    setExporting(true);
    setError(null);
    setExportedFiles([]);

    try {
      const data = await exportLogo({
        image_base64: variant.image_data,
        product_name: productName.trim(),
      });
      setExportedFiles(data.files);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Export failed");
    } finally {
      setExporting(false);
    }
  }, [selectedVariant, variants, productName]);

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-8">
      <h2 className="text-2xl font-bold text-gray-900">Logo Generator</h2>

      {/* Product Name */}
      <div className="space-y-2">
        <label className="block text-sm font-medium text-gray-700">
          Product Name
        </label>
        <input
          type="text"
          value={productName}
          onChange={(e) => setProductName(e.target.value)}
          placeholder="Enter product name..."
          className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        />
      </div>

      {/* Description */}
      <div className="space-y-2">
        <label className="block text-sm font-medium text-gray-700">
          Description (optional)
        </label>
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Describe your product..."
          rows={3}
          className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        />
      </div>

      {/* Style Picker */}
      <div className="space-y-2">
        <label className="block text-sm font-medium text-gray-700">Style</label>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
          {STYLES.map((style) => (
            <button
              key={style.id}
              onClick={() => setSelectedStyle(style.id)}
              className={`p-3 rounded-lg border-2 text-left transition-colors ${
                selectedStyle === style.id
                  ? "border-blue-500 bg-blue-50 text-blue-900"
                  : "border-gray-200 hover:border-gray-300"
              }`}
            >
              <div className="font-medium text-sm">{style.label}</div>
              <div className="text-xs text-gray-500 mt-1">{style.desc}</div>
            </button>
          ))}
        </div>
      </div>

      {/* Color Picker */}
      <div className="space-y-2">
        <label className="block text-sm font-medium text-gray-700">Colors</label>
        <div className="flex gap-4">
          {([0, 1] as const).map((idx) => (
            <div key={idx} className="flex items-center gap-2">
              <input
                type="color"
                value={colors[idx]}
                onChange={(e) => handleColorChange(idx, e.target.value)}
                className="w-10 h-10 rounded cursor-pointer border border-gray-300"
              />
              <span className="text-sm text-gray-600 font-mono">{colors[idx]}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Generate Button */}
      <button
        onClick={generate}
        disabled={!productName.trim() || loading}
        className="w-full py-3 px-4 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {loading ? "Generating..." : "Generate Logo"}
      </button>

      <ErrorDisplay message={error} />

      {/* Variant Grid */}
      {variants.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-lg font-semibold text-gray-900">
            Select a Variant
          </h3>
          <div className="grid grid-cols-2 gap-4">
            {variants.map((variant) => (
              <button
                key={variant.index}
                onClick={() => setSelectedVariant(variant.index)}
                className={`relative rounded-lg overflow-hidden border-2 transition-all ${
                  selectedVariant === variant.index
                    ? "border-blue-500 ring-2 ring-blue-300"
                    : "border-gray-200 hover:border-gray-300"
                }`}
              >
                <img
                  src={
                    variant.is_url
                      ? variant.image_data
                      : `data:image/png;base64,${variant.image_data}`
                  }
                  alt={`Logo variant ${variant.index + 1}`}
                  className="w-full aspect-square object-contain bg-gray-50"
                />
                {selectedVariant === variant.index && (
                  <div className="absolute top-2 right-2 w-6 h-6 bg-blue-500 rounded-full flex items-center justify-center">
                    <svg
                      className="w-4 h-4 text-white"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M5 13l4 4L19 7"
                      />
                    </svg>
                  </div>
                )}
                <div className="p-2 text-center text-sm text-gray-600">
                  Variant {variant.index + 1}
                </div>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Export Button */}
      {selectedVariant !== null && (
        <button
          onClick={doExport}
          disabled={exporting}
          className="w-full py-3 px-4 bg-green-600 text-white rounded-lg font-medium hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {exporting ? "Exporting..." : "Export Icons"}
        </button>
      )}

      {/* Export Results */}
      {exportedFiles.length > 0 && (
        <div className="p-4 bg-green-50 border border-green-200 rounded-lg space-y-3">
          <h3 className="text-lg font-semibold text-green-800">
            Export Complete
          </h3>
          <ul className="space-y-2">
            {exportedFiles.map((file) => (
              <li
                key={file.path}
                className="flex items-center justify-between text-sm"
              >
                <span className="text-green-700 font-mono truncate">
                  {file.path}
                </span>
                <span className="text-green-600 ml-2 whitespace-nowrap">
                  {file.format.toUpperCase()}
                  {file.dimensions
                    ? ` ${file.dimensions[0]}x${file.dimensions[1]}`
                    : ""}
                  {" "}
                  ({Math.round(file.size_bytes / 1024)}KB)
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
