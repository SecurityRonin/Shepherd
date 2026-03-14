import React from "react";

export interface Plugin {
  id: string;
  name: string;
  description: string;
  detected: boolean;
  installUrl?: string;
}

interface PluginCardProps {
  plugin: Plugin;
}

export const PluginCard: React.FC<PluginCardProps> = ({ plugin }) => {
  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700" data-testid={`plugin-${plugin.id}`}>
      <div className="flex items-start justify-between">
        <div>
          <h3 className="text-sm font-semibold text-white">{plugin.name}</h3>
          <p className="text-xs text-gray-400 mt-1">{plugin.description}</p>
        </div>
        {plugin.detected ? (
          <span
            className="px-2 py-0.5 rounded text-xs bg-green-700 text-green-100"
            data-testid={`badge-detected-${plugin.id}`}
          >
            Detected
          </span>
        ) : (
          plugin.installUrl && (
            <a
              href={plugin.installUrl}
              target="_blank"
              rel="noreferrer"
              className="text-xs text-blue-400 hover:underline"
              data-testid={`install-link-${plugin.id}`}
            >
              Install
            </a>
          )
        )}
      </div>
    </div>
  );
};
