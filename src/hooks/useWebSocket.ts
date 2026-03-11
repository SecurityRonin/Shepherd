import { useEffect, useRef } from "react";
import {
  createWsClient,
  type WsClient,
  type ConnectionStatus,
} from "../lib/ws";
import type { ServerEvent } from "../types";

const DEFAULT_PORT = 9876;
const WS_PATH = "/ws";

function getServerUrl(): string {
  return `ws://127.0.0.1:${DEFAULT_PORT}${WS_PATH}`;
}

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
    const client = createWsClient({
      url: getServerUrl(),
      onEvent: (event) => onEventRef.current(event),
      onStatusChange: (s) => onStatusRef.current(s),
    });
    clientRef.current = client;
    client.connect();
    client.send({ type: "subscribe", data: null });
    return () => {
      client.disconnect();
      clientRef.current = null;
    };
  }, []);

  return clientRef;
}
