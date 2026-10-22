import path from 'node:path'
import { defineConfig } from 'vitest/config'

// End-to-end tests against a running Rust API (see scripts/e2e.mjs).
export default defineConfig({
  resolve: {
    alias: { '@': path.resolve(__dirname, 'src') },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.e2e.test.ts'],
    testTimeout: 120_000,
  },
})
