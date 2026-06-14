const { fetchUrl } = require('../lib/webtools')

async function main() {
  const payload = process.argv[2] ? JSON.parse(process.argv[2]) : {}
  const url = String(payload.url || '').trim()
  const title = String(payload.title || '').trim()
  if (!url) {
    throw new Error('missing url')
  }
  const content = await fetchUrl(url)
  process.stdout.write(JSON.stringify({
    ok: true,
    url,
    title,
    content,
    summary: `[网页内容：${title || url}]\n\n${String(content || '').substring(0, 8000)}`,
  }))
}

main().catch((error) => {
  process.stderr.write(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
