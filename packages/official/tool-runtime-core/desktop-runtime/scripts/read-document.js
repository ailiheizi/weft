const path = require('path')
const { findDocumentByTitle, readDocumentFile, formatForClaude } = require('../lib/doc-reader')

async function main() {
  const payload = process.argv[2] ? JSON.parse(process.argv[2]) : {}
  const windowTitle = String(payload.windowTitle || payload.window_title || '').trim()
  const filePath = await findDocumentByTitle(windowTitle).catch(() => null)
  if (!filePath) {
    throw new Error('未找到正在打开的文档，请确认 WPS/Office 中有打开的文件。')
  }
  const result = await readDocumentFile(filePath)
  if (!result?.ok) {
    throw new Error(`文档读取失败: ${result?.error || '未知错误'}`)
  }
  process.stdout.write(JSON.stringify({
    ok: true,
    path: filePath,
    filename: path.basename(filePath),
    chars: Number(result.chars || 0),
    content: String(result.content || ''),
    summary: formatForClaude(filePath, String(result.content || ''), 10000),
  }))
}

main().catch((error) => {
  process.stderr.write(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
