// Development mode: start the Rust API (port 8080) and the Vite dev server (port 5173,
// proxying /api to the API). Stop both with Ctrl+C. Usage: node scripts/dev.mjs
import { spawn } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const procs = [
  spawn('cargo run --release -p api-server --bin airship-api -- --port 8080', { cwd: root, stdio: 'inherit', shell: true }),
  spawn('pnpm run dev', { cwd: join(root, 'apps', 'web'), stdio: 'inherit', shell: true }),
]
const stop = () => {
  for (const p of procs) p.kill()
  process.exit(0)
}
process.on('SIGINT', stop)
process.on('SIGTERM', stop)
for (const p of procs) p.on('exit', (code) => code && code !== 0 && stop())
