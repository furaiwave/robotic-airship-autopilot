// Build and start the Rust API on a free port, run the frontend end-to-end tests
// against it, then stop the server. Usage: node scripts/e2e.mjs
import { spawn, spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const port = process.env.E2E_PORT ?? '8099'
const run = (cmd, args, opts = {}) => {
  const r = spawnSync([cmd, ...args].join(' '), { stdio: 'inherit', cwd: root, shell: true, ...opts })
  if (r.status !== 0) process.exit(r.status ?? 1)
}

run('cargo', ['build', '--release', '-p', 'api-server'])
const exe = join(root, 'target', 'release', process.platform === 'win32' ? 'airship-api.exe' : 'airship-api')
if (!existsSync(exe)) {
  console.error(`missing ${exe}`)
  process.exit(1)
}
const server = spawn(exe, ['--port', port], { cwd: root, stdio: ['ignore', 'inherit', 'inherit'] })
const url = `http://127.0.0.1:${port}`
let up = false
for (let i = 0; i < 100 && !up; i++) {
  try {
    up = (await fetch(`${url}/api/health`)).ok
  } catch {
    await new Promise((r) => setTimeout(r, 200))
  }
}
if (!up) {
  server.kill()
  console.error('API did not start')
  process.exit(1)
}
const r = spawnSync('pnpm run test:e2e', { cwd: join(root, 'apps', 'web'), stdio: 'inherit', shell: true, env: { ...process.env, E2E_API_URL: url } })
server.kill()
process.exit(r.status ?? 1)
