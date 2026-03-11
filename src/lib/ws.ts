import type { ServerEvent, ClientEvent } from "../types";

export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "reconnecting";
export type ServerEventHandler = (event: ServerEvent) => void;
export type StatusChangeHandler = (status: ConnectionStatus) => void;

export interface WsClientOptions {
  url: string;
  onEvent: ServerEventHandler;
  onStatusChange: StatusChangeHandler;
  maxReconnectAttempts?: number;
  initialReconnectDelay?: number;
}

export interface WsClient {
  connect(): void;
  disconnect(): void;
  send(event: ClientEvent): void;
  getStatus(): ConnectionStatus;
}

export function createWsClient(options: WsClientOptions): WsClient {
  const {
    url,
    onEvent,
    onStatusChange,
    maxReconnectAttempts = 0,
    initialReconnectDelay = 1000,
  } = options;

  let ws: WebSocket | null = null;
  let status: ConnectionStatus = "disconnected";
  let reconnectAttempts = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let intentionalClose = false;
  let messageQueue: ClientEvent[] = [];

  function setStatus(newStatus: ConnectionStatus): void {
    if (status !== newStatus) {
      status = newStatus;
      onStatusChange(newStatus);
    }
  }

  function flushQueue(): void {
    while (messageQueue.length > 0 && ws?.readyState === WebSocket.OPEN) {
      const event = messageQueue.shift()!;
      ws.send(JSON.stringify(event));
    }
  }

  function scheduleReconnect(): void {
    if (intentionalClose) return;
    if (maxReconnectAttempts > 0 && reconnectAttempts >= maxReconnectAttempts) {
      setStatus("disconnected");
      return;
    }
    setStatus("reconnecting");
    const delay = Math.min(
      initialReconnectDelay * Math.pow(2, reconnectAttempts),
      30000,
    );
    reconnectAttempts++;
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, delay);
  }

  function connect(): void {
    if (
      ws?.readyState === WebSocket.OPEN ||
      ws?.readyState === WebSocket.CONNECTING
    ) {
      return;
    }

    intentionalClose = false;
    setStatus("connecting");

    try {
      ws = new WebSocket(url);
    } catch {
      scheduleReconnect();
      return;
    }

    ws.onopen = () => {
      reconnectAttempts = 0;
      setStatus("connected");
      flushQueue();
    };

    ws.onmessage = (msgEvent: MessageEvent) => {
      try {
        const parsed = JSON.parse(msgEvent.data as string) as ServerEvent;
        onEvent(parsed);
      } catch (err) {
        console.error("[ws] Failed to parse server event:", err, msgEvent.data);
      }
    };

    ws.onclose = () => {
      ws = null;
      if (!intentionalClose) {
        scheduleReconnect();
      } else {
        setStatus("disconnected");
      }
    };

    ws.onerror = () => {
      // Error handling is done in onclose
    };
  }

  function disconnect(): void {
    intentionalClose = true;
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (ws) {
      ws.close(1000, "Client disconnect");
      ws = null;
    }
    messageQueue = [];
    setStatus("disconnected");
  }

  function send(event: ClientEvent): void {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(event));
    } else {
      messageQueue.push(event);
    }
  }

  function getStatus(): ConnectionStatus {
    return status;
  }

  return { connect, disconnect, send, getStatus };
}
