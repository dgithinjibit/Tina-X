// React hook: connect to the Rust WebSocket and keep a rolling window of reflex samples.
//
// For a junior dev: this isolates ALL the WebSocket/side-effect logic so components stay
// declarative. If the socket drops, we retry after a short delay. The pure windowing math
// lives in telemetry.ts (and is unit-tested there), so this hook stays thin.

import { useEffect, useRef, useState } from "react";
import { wsUrl, type ReflexSample, type TelemetryMsg } from "./api";
import { pushWindow } from "./telemetry";

export type ConnState = "connecting" | "open" | "closed";

/** A brain decision as carried on the wire (the tag stripped off). */
export interface BrainDecision {
  id: number;
  query: string;
  results: string[];
  verified: boolean;
}

export function useTelemetry(windowSize = 120) {
  const [samples, setSamples] = useState<ReflexSample[]>([]);
  // The most recent slow-brain decisions (newest first), for the decision panel.
  const [decisions, setDecisions] = useState<BrainDecision[]>([]);
  const [conn, setConn] = useState<ConnState>("connecting");
  // Keep the socket in a ref so re-renders don't reopen it.
  const socketRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    let closedByUs = false;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;

    function connect() {
      setConn("connecting");
      const ws = new WebSocket(wsUrl());
      socketRef.current = ws;

      ws.onopen = () => setConn("open");

      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data as string) as TelemetryMsg;
          if (msg.type === "reflex") {
            // Strip the tag so we store a clean ReflexSample.
            const { type: _t, ...sample } = msg;
            setSamples((prev) => pushWindow(prev, sample as ReflexSample, windowSize));
          } else if (msg.type === "decision") {
            // Keep the newest few decisions (newest first); the slow brain is low-rate.
            const { type: _t, ...decision } = msg;
            setDecisions((prev) => [decision as BrainDecision, ...prev].slice(0, 8));
          }
        } catch {
          // Ignore malformed frames rather than crashing the UI.
        }
      };

      ws.onclose = () => {
        setConn("closed");
        // Auto-reconnect unless we intentionally tore down (component unmount).
        if (!closedByUs) retryTimer = setTimeout(connect, 1500);
      };

      ws.onerror = () => ws.close();
    }

    connect();

    return () => {
      closedByUs = true;
      if (retryTimer) clearTimeout(retryTimer);
      socketRef.current?.close();
    };
  }, [windowSize]);

  return { samples, decisions, conn };
}
