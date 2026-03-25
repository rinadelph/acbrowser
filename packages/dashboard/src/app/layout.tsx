import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "agent-browser",
  description: "Observability dashboard for agent-browser",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
