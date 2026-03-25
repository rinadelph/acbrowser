"use client";

import { useEffect, useRef, useState } from "react";
import type { ActivityEvent } from "@/hooks/useStreamConnection";

interface ActivityFeedProps {
  events: ActivityEvent[];
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString("en-US", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function highlightRefs(text: string): React.ReactNode {
  const parts = text.split(/(@e\d+)/g);
  return parts.map((part, i) =>
    part.match(/^@e\d+$/) ? (
      <span key={i} className="text-[var(--accent)] font-mono font-semibold">
        {part}
      </span>
    ) : (
      part
    ),
  );
}

function CommandEntry({ event }: { event: ActivityEvent & { type: "command" } }) {
  const [expanded, setExpanded] = useState(false);

  const label = event.action;
  const hasParams =
    event.params &&
    Object.keys(event.params).filter((k) => k !== "action" && k !== "id")
      .length > 0;

  return (
    <div className="border-b border-[var(--border)] py-1.5 px-3">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full text-left flex items-center gap-2 text-xs"
      >
        <span className="text-[var(--text-muted)] font-mono shrink-0">
          {formatTime(event.timestamp)}
        </span>
        <span className="text-[var(--accent)] shrink-0">{"->"}</span>
        <span className="font-mono font-semibold truncate">
          {highlightRefs(label)}
        </span>
        {hasParams && (
          <span className="text-[var(--text-muted)] ml-auto shrink-0">
            {expanded ? "-" : "+"}
          </span>
        )}
      </button>
      {expanded && hasParams && (
        <pre className="text-[10px] text-[var(--text-muted)] mt-1 ml-16 overflow-x-auto max-h-32 overflow-y-auto">
          {JSON.stringify(
            Object.fromEntries(
              Object.entries(event.params).filter(
                ([k]) => k !== "action" && k !== "id",
              ),
            ),
            null,
            2,
          )}
        </pre>
      )}
    </div>
  );
}

function ResultEntry({ event }: { event: ActivityEvent & { type: "result" } }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="border-b border-[var(--border)] py-1.5 px-3">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full text-left flex items-center gap-2 text-xs"
      >
        <span className="text-[var(--text-muted)] font-mono shrink-0">
          {formatTime(event.timestamp)}
        </span>
        <span
          className={`shrink-0 ${event.success ? "text-[var(--success)]" : "text-[var(--error)]"}`}
        >
          {"<-"}
        </span>
        <span className="font-mono truncate">
          {event.action}
          <span className="text-[var(--text-muted)] ml-1">
            {event.duration_ms}ms
          </span>
        </span>
        <span className="text-[var(--text-muted)] ml-auto shrink-0">
          {expanded ? "-" : "+"}
        </span>
      </button>
      {expanded && event.data != null && (
        <pre className="text-[10px] text-[var(--text-muted)] mt-1 ml-16 overflow-x-auto max-h-48 overflow-y-auto">
          {typeof event.data === "string"
            ? event.data
            : JSON.stringify(event.data, null, 2)}
        </pre>
      )}
    </div>
  );
}

function ConsoleEntry({
  event,
}: {
  event: ActivityEvent & { type: "console" };
}) {
  const levelColors: Record<string, string> = {
    error: "text-[var(--error)]",
    warn: "text-[var(--warning)]",
    warning: "text-[var(--warning)]",
    info: "text-[var(--accent)]",
    log: "text-[var(--text-muted)]",
  };

  return (
    <div className="border-b border-[var(--border)] py-1.5 px-3 flex items-start gap-2 text-xs">
      <span className="text-[var(--text-muted)] font-mono shrink-0">
        {formatTime(event.timestamp)}
      </span>
      <span className={`shrink-0 ${levelColors[event.level] ?? "text-[var(--text-muted)]"}`}>
        [{event.level}]
      </span>
      <span className="font-mono truncate">{event.text}</span>
    </div>
  );
}

export function ActivityFeed({ events }: ActivityFeedProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const autoScrollRef = useRef(true);

  useEffect(() => {
    if (autoScrollRef.current) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [events.length]);

  const handleScroll = () => {
    const el = containerRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    autoScrollRef.current = atBottom;
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center px-3 py-2 border-b border-[var(--border)] shrink-0">
        <span className="text-xs font-semibold text-[var(--text-muted)] uppercase tracking-wide">
          Activity
        </span>
        <span className="text-xs text-[var(--text-muted)] ml-auto">
          {events.length} events
        </span>
      </div>

      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto min-h-0"
      >
        {events.length === 0 ? (
          <div className="text-[var(--text-muted)] text-xs text-center py-8">
            Waiting for events...
          </div>
        ) : (
          events.map((event, i) => {
            switch (event.type) {
              case "command":
                return <CommandEntry key={i} event={event} />;
              case "result":
                return <ResultEntry key={i} event={event} />;
              case "console":
                return <ConsoleEntry key={i} event={event} />;
            }
          })
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
