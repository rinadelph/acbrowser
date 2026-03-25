"use client";

import { useCallback, useEffect, useRef } from "react";

interface ViewportProps {
  frame: string | null;
  viewportWidth: number;
  viewportHeight: number;
  browserConnected: boolean;
  screencasting: boolean;
}

export function Viewport({
  frame,
  viewportWidth,
  viewportHeight,
  browserConnected,
  screencasting,
}: ViewportProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);

  const drawFrame = useCallback(
    (base64: string) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      if (!imgRef.current) {
        imgRef.current = new Image();
      }
      const img = imgRef.current;
      img.onload = () => {
        canvas.width = img.naturalWidth;
        canvas.height = img.naturalHeight;
        const ctx = canvas.getContext("2d");
        if (ctx) {
          ctx.drawImage(img, 0, 0);
        }
      };
      img.src = `data:image/jpeg;base64,${base64}`;
    },
    [],
  );

  useEffect(() => {
    if (frame) {
      drawFrame(frame);
    }
  }, [frame, drawFrame]);

  const aspectRatio = viewportWidth / viewportHeight;

  return (
    <div ref={containerRef} className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-[var(--border)] shrink-0">
        <div
          className={`w-2 h-2 rounded-full ${
            browserConnected ? "bg-[var(--success)]" : "bg-[var(--error)]"
          }`}
        />
        <span className="text-xs text-[var(--text-muted)]">
          {browserConnected
            ? screencasting
              ? "Live"
              : "Connected"
            : "Disconnected"}
        </span>
        <span className="text-xs text-[var(--text-muted)] ml-auto">
          {viewportWidth} x {viewportHeight}
        </span>
      </div>

      <div className="flex-1 flex items-center justify-center p-2 min-h-0">
        {frame ? (
          <canvas
            ref={canvasRef}
            className="max-w-full max-h-full rounded"
            style={{ aspectRatio }}
          />
        ) : (
          <div className="text-[var(--text-muted)] text-sm text-center">
            {browserConnected
              ? "Waiting for frames..."
              : "No browser connected"}
          </div>
        )}
      </div>
    </div>
  );
}
