import { defineConfig } from "@playwright/test";

const port = 5183;

export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    trace: "retain-on-failure",
  },
  webServer: {
    command: `yarn vite --port ${port} --strictPort`,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: !process.env.CI,
    env: {
      VITE_RPC_URL: process.env.VITE_RPC_URL ?? "http://127.0.0.1:8899",
      VITE_CHAIN: process.env.VITE_CHAIN ?? "solana:devnet",
    },
  },
});
