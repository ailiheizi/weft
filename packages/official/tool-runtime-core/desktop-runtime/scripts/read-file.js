const fs = require('fs')
const path = require('path')

async function main() {
  const payload = process.argv[2] ? JSON.parse(process.argv[2]) : {}
  const filePath = String(payload.path || '').trim()
  if (!filePath) {
    throw new Error('请提供文件路径')
  }
  if (!fs.existsSync(filePath)) {
    throw new Error(`文件不存在: ${filePath}`)
  }

  const unsupportedDocumentExts = new Set(['.docx', '.doc', '.pdf', '.xlsx', '.xls', '.pptx', '.ppt', '.csv', '.ods'])
  const ext = path.extname(filePath).toLowerCase()
  if (unsupportedDocumentExts.has(ext)) {
    throw new Error(`read_file 只读取纯文本文件；请用对应文档 skill 读取 ${ext || '该'} 文件`)
  }

  const text = fs.readFileSync(filePath, 'utf-8')
  const truncated = text.length > 12000
    ? text.substring(0, 12000) + `\n\n[…已截断，完整文件共${text.length}字]`
    : text
  process.stdout.write(JSON.stringify({
    ok: true,
    path: filePath,
    content: text,
    summary: `[文件：${path.basename(filePath)}]\n\n${truncated}`,
  }))
}

main().catch((error) => {
  process.stderr.write(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
