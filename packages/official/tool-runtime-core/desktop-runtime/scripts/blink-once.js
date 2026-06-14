const screenVision = require('../lib/screen_vision')

async function main() {
  const originalLog = console.log
  const originalWarn = console.warn
  console.log = (...args) => process.stderr.write(`${args.join(' ')}\n`)
  console.warn = (...args) => process.stderr.write(`${args.join(' ')}\n`)
  const payloadArg = process.argv[2] || ''
  const payload = payloadArg
    ? JSON.parse(payloadArg.startsWith('payload_b64:')
      ? Buffer.from(payloadArg.slice('payload_b64:'.length), 'base64').toString('utf8')
      : payloadArg)
    : {}
  const task = String(payload.task || '').trim() || 'look at the current screen'
  const forceSystem = payload.forceSystem !== false
  const screenIndex = Number(payload.screenIndex || 1)
  const prevSummaries = Array.isArray(payload.prevSummaries)
    ? payload.prevSummaries.map((entry) => String(entry || ''))
    : []

  try {
    const result = await screenVision.blinkOnce(task, {
      forceSystem,
      screenIndex,
      prevSummaries,
    })

    const output = {
      ok: true,
      content: String(result?.content || ''),
      content_b64: Buffer.from(String(result?.content || ''), 'utf8').toString('base64'),
      useBrowser: !!result?.useBrowser,
    }
    process.stdout.write(JSON.stringify({
      ok: true,
      payload_b64: Buffer.from(JSON.stringify(output), 'utf8').toString('base64'),
    }))
  } catch (error) {
    const message = error?.response?.data
      ? `${error.message}: ${JSON.stringify(error.response.data)}`
      : (error instanceof Error ? error.message : String(error))
    const output = {
      ok: false,
      error: message,
      error_b64: Buffer.from(message, 'utf8').toString('base64'),
      content: '',
      content_b64: '',
      useBrowser: false,
    }
    process.stdout.write(JSON.stringify({
      ok: false,
      payload_b64: Buffer.from(JSON.stringify(output), 'utf8').toString('base64'),
    }))
    process.exitCode = 1
  }
  console.log = originalLog
  console.warn = originalWarn
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error)
  const output = {
    ok: false,
    error: message,
    error_b64: Buffer.from(message, 'utf8').toString('base64'),
    content: '',
    content_b64: '',
    useBrowser: false,
  }
  process.stdout.write(JSON.stringify({
    ok: false,
    payload_b64: Buffer.from(JSON.stringify(output), 'utf8').toString('base64'),
  }))
  process.exit(1)
})
