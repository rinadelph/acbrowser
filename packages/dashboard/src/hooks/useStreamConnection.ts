"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export interface FrameMessage {
  type: "frame";
  data: string;
  metadata: {
    offsetTop: number;
    pageScaleFactor: number;
    deviceWidth: number;
    deviceHeight: number;
    scrollOffsetX: number;
    scrollOffsetY: number;
    timestamp: number;
  };
}

export interface StatusMessage {
  type: "status";
  connected: boolean;
  screencasting: boolean;
  viewportWidth: number;
  viewportHeight: number;
}

export interface CommandMessage {
  type: "command";
  action: string;
  id: string;
  params: Record<string, unknown>;
  timestamp: number;
}

export interface ResultMessage {
  type: "result";
  id: string;
  action: string;
  success: boolean;
  data: unknown;
  duration_ms: number;
  timestamp: number;
}

export interface ConsoleMessage {
  type: "console";
  level: string;
  text: string;
  timestamp: number;
}

export interface ErrorMessage {
  type: "error";
  message: string;
}

export type StreamMessage =
  | FrameMessage
  | StatusMessage
  | CommandMessage
  | ResultMessage
  | ConsoleMessage
  | ErrorMessage;

export type ActivityEvent = CommandMessage | ResultMessage | ConsoleMessage;

export interface StreamState {
  connected: boolean;
  browserConnected: boolean;
  screencasting: boolean;
  viewportWidth: number;
  viewportHeight: number;
  currentFrame: string | null;
  events: ActivityEvent[];
}

const MAX_EVENTS = 500;

export function useStreamConnection(port: number = 9223) {
  const [state, setState] = useState<StreamState>({
    connected: false,
    browserConnected: false,
    screencasting: false,
    viewportWidth: 1280,
    viewportHeight: 720,
    currentFrame: null,
    events: [],
  });

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const eventsRef = useRef<ActivityEvent[]>([]);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    const ws = new WebSocket(`ws://localhost:${port}`);
    wsRef.current = ws;

    ws.onopen = () => {
      setState((prev) => ({ ...prev, connected: true }));
    };

    ws.onclose = () => {
      setState((prev) => ({ ...prev, connected: false }));
      reconnectTimerRef.current = setTimeout(connect, 2000);
    };

    ws.onerror = () => {
      ws.close();
    };

    ws.onmessage = (event) => {
      let msg: StreamMessage;
      try {
        msg = JSON.parse(event.data);
      } catch {
        return;
      }

      switch (msg.type) {
        case "frame":
          setState((prev) => ({
            ...prev,
            currentFrame: msg.data,
          }));
          break;

        case "status":
          setState((prev) => ({
            ...prev,
            browserConnected: msg.connected,
            screencasting: msg.screencasting,
            viewportWidth: msg.viewportWidth,
            viewportHeight: msg.viewportHeight,
          }));
          break;

        case "command":
        case "result":
        case "console": {
          const updated = [...eventsRef.current, msg].slice(-MAX_EVENTS);
          eventsRef.current = updated;
          setState((prev) => ({
            ...prev,
            events: updated,
          }));
          break;
        }

        case "error":
          break;
      }
    };
  }, [port]);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
    };
  }, [connect]);

  return state;
}
