const fs = require('fs')
const http = require('http')
const path = require('path')
const { spawn } = require('child_process')

const ROOT = path.resolve(__dirname, '..')
const DATA_DIR = path.join(ROOT, 'data')
const STATE_PATH = path.join(DATA_DIR, 'meeting-daemon-state.json')
const DEFAULT_PORT = Number(process.env.WEFT_RTONE_MEETING_DAEMON_PORT || 43183)
const action = String(process.argv[2] || 'status').trim().toLowerCase()
const outputDir = String(process.argv[3] || '').trim()

fs.mkdirSync(DATA_DIR, { recursive: true })

function request(method, route, payload) {
  return new Promise((resolve, reject) => {
    const body = payload ? JSON.stringify(payload) : ''
    const req = http.request(
      {
        host: '127.0.0.1',
        port: DEFAULT_PORT,
        path: route,
        method,
        headers: {
          'Content-Type': 'application/json; charset=utf-8',
          'Content-Length': Buffer.byteLength(body),
        },
      },
      (res) => {
        let raw = ''
        res.setEncoding('utf8')
        res.on('data', (chunk) => {
          raw += chunk
        })
        res.on('end', () => {
          try {
            const parsed = raw.trim() ? JSON.parse(raw) : {}
            if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
              resolve(parsed)
              return
            }
            reject(new Error(parsed.error || `http ${res.statusCode}`))
          } catch (error) {
            reject(error)
          }
        })
      },
    )
    req.on('error', reject)
    req.end(body)
  })
}

async function isHealthy() {
  try {
    await request('GET', '/health')
    return true
  } catch {
    return false
  }
}

async function ensureDaemon() {
  if (await isHealthy()) return

  const child = spawn(process.execPath, [path.join(__dirname, 'meeting-daemon.js')], {
    detached: true,
    stdio: 'ignore',
    env: process.env,
  })
  child.unref()

  const deadline = Date.now() + 15000
  while (Date.now() < deadline) {
    if (await isHealthy()) return
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error('meeting daemon did not become ready')
}

async function main() {
  await ensureDaemon()

  let result
  if (action === 'start') {
    result = await request('POST', '/start', outputDir ? { outputDir } : {})
  } else if (action === 'stop') {
    result = await request('POST', '/stop', {})
  } else {
    result = await request('GET', '/status')
  }

  fs.writeFileSync(
    STATE_PATH,
    JSON.stringify(
      {
        port: DEFAULT_PORT,
        pid: process.pid,
        updatedAt: Date.now(),
      },
      null,
      2,
    ),
    'utf8',
  )
  process.stdout.write(JSON.stringify(result))
}

main().catch((error) => {
  process.stderr.write(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
