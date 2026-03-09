import type { NextConfig } from "next";
import path from "node:path";

const nextConfig: NextConfig = {
  outputFileTracingRoot: path.resolve(import.meta.dirname, "../../"),
  serverExternalPackages: ["@sparticuz/chromium"],
  outputFileTracingIncludes: {
    "/**": ["./node_modules/@sparticuz/chromium/**"],
  },
};

export default nextConfig;
