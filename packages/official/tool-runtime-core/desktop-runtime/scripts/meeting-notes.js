const { execute } = require('../lib/deepseek')

async function main() {
  const payload = process.argv[2] ? JSON.parse(process.argv[2]) : {}
  const prompt = String(payload.prompt || '').trim()
  if (!prompt) {
    throw new Error('missing prompt')
  }

  const result = await execute('create_meeting_doc', prompt, null)
  if (!result || result.success !== true) {
    throw new Error(result?.error || 'meeting notes generation failed')
  }

  process.stdout.write(JSON.stringify({
    ok: true,
    meeting_title: String(payload.meetingTitle || ''),
    transcript: String(payload.transcript || ''),
    content: String(result.content || ''),
    files: Array.isArray(result.files) ? result.files : [],
  }))
}

main().catch((error) => {
  process.stderr.write(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
