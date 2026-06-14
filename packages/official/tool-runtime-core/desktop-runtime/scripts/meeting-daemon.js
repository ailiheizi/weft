const fs = require('fs')
const http = require('http')
const path = require('path')

const recorder = require('../lib/recorder')
const { transcribeWav } = require('../lib/meetingTranscribe')

const ROOT = path.resolve(__dirname, '..')
const DATA_DIR = path.join(ROOT, 'data')
const STATE_PATH = path.join(DATA_DIR, 'meeting-daemon-state.json')
const PORT = Number(process.env.WEFT_RTONE_MEETING_DAEMON_PORT || 43183)

fs.mkdirSync(DATA_DIR, { recursive: true })

function writeState() {
  fs.writeFileSync(
    STATE_PATH,
    JSON.stringify(
      {
        port: PORT,
        pid: process.pid,
        updatedAt: Date.now(),
      },
      null,
      2,
    ),
    'utf8',
  )
}

function readJson(req) {
  return new Promise((resolve, reject) => {
    let raw = ''
    req.setEncoding('utf8')
    req.on('data', (chunk) => {
      raw += chunk
    })
    req.on('end', () => {
      if (!raw.trim()) {
        resolve({})
        return
      }
      try {
        resolve(JSON.parse(raw))
      } catch (error) {
        reject(error)
      }
    })
    req.on('error', reject)
  })
}

function sendJson(res, statusCode, payload) {
  const body = JSON.stringify(payload)
  res.writeHead(statusCode, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(body),
  })
  res.end(body)
}

const server = http.createServer(async (req, res) => {
  try {
    if (!req.url) {
      sendJson(res, 404, { ok: false, error: 'missing url' })
      return
    }

    if (req.method === 'GET' && req.url === '/health') {
      sendJson(res, 200, { ok: true, port: PORT, pid: process.pid })
      return
    }

    if (req.method === 'GET' && req.url === '/status') {
      sendJson(res, 200, {
        ok: true,
        recording: recorder.isRecording(),
        hasActiveSession: recorder.hasActiveSession(),
        elapsedSeconds: recorder.elapsedSeconds(),
      })
      return
    }

    if (req.method === 'POST' && req.url === '/start') {
      const payload = await readJson(req)
      if (payload && typeof payload === 'object' && typeof payload.outputDir === 'string' && payload.outputDir.trim()) {
        process.env.DEEPSEEK_WORK_DIR = payload.outputDir.trim()
      }
      const wavPath = await recorder.startRecording()
      sendJson(res, 200, {
        ok: true,
        action: 'start',
        path: wavPath,
        recording: true,
      })
      return
    }

    if (req.method === 'POST' && req.url === '/stop') {
      const stopped = await recorder.stopRecording()
      const transcript = await transcribeWav(stopped.path)
      sendJson(res, 200, {
        ok: true,
        action: 'stop',
        path: stopped.path,
        duration: stopped.duration,
        transcript,
      })
      return
    }

    sendJson(res, 404, { ok: false, error: `unknown route: ${req.method} ${req.url}` })
  } catch (error) {
    sendJson(res, 500, {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
})

server.listen(PORT, '127.0.0.1', () => {
  writeState()
})

const shutdown = () => {
  try {
    recorder.forceReset()
  } catch {}
  try {
    if (fs.existsSync(STATE_PATH)) {
      fs.unlinkSync(STATE_PATH)
    }
  } catch {}
  process.exit(0)
}

process.on('SIGINT', shutdown)
process.on('SIGTERM', shutdown)
