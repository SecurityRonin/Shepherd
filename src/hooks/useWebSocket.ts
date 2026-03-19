import { useEffect, useRef } from "react";
import {
  createWsClient,
  type WsClient,
  type ConnectionStatus,
} from "../lib/ws";
import type { ServerEvent } from "../types";
import { getServerPort } from "../lib/tauri";

export function useWebSocket(
  onEvent: (event: ServerEvent) => void,
  onStatusChange: (status: ConnectionStatus) => void,
): React.MutableRefObject<WsClient | null> {
  const clientRef = useRef<WsClient | null>(null);
  const onEventRef = useRef(onEvent);
  const onStatusRef = useRef(onStatusChange);
  onEventRef.current = onEvent;
  onStatusRef.current = onStatusChange;

  useEffect(() => {
    let cancelled = false;

    getServerPort().then((port) => {
      if (cancelled) return;
      const wsUrl = `ws://127.0.0.1:${port}/ws`;
      const client = createWsClient({
        url: wsUrl,
        onEvent: (event) => onEventRef.current(event),
        onStatusChange: (s) => onStatusRef.current(s),
      });
      clientRef.current = client;
      client.connect();
      client.send({ type: "subscribe", data: null });
    });

    return () => {
      cancelled = true;
      if (clientRef.current) {
        clientRef.current.disconnect();
        clientRef.current = null;
      }
    };
  }, []);

  return clientRef;
}
