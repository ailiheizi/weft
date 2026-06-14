/**
 * Desktop document reader.
 * Reads DOCX/PDF/XLSX/PPTX and plain text content directly.
 */
'use strict'

const { execSync, execFile } = require('child_process')
const fs = require('fs')
const path = require('path')
const os = require('os')

const READER_PY = String.raw`
import sys, json, os

def read_docx(fp):
    from docx import Document
    doc = Document(fp)
    parts = []
    for p in doc.paragraphs:
        t = p.text.strip()
        if t:
            style = p.style.name if p.style else ''
            prefix = '# ' if 'Heading 1' in style else ('## ' if 'Heading' in style else '')
            parts.append(prefix + t)
    tables = []
    for tbl in doc.tables:
        rows = []
        for row in tbl.rows:
            rows.append(' | '.join(c.text.strip() for c in row.cells))
        tables.append('\n'.join(rows))
    result = '\n'.join(parts)
    if tables:
        result += '\n\n[表格]\n' + '\n\n'.join(tables)
    return result

def read_pdf(fp):
    import fitz
    doc = fitz.open(fp)
    pages = []
    for i, page in enumerate(doc):
        text = page.get_text().strip()
        if text:
            pages.append(f'[第{i+1}页]\n{text}')
    return '\n\n'.join(pages)

def read_xlsx(fp):
    import openpyxl
    wb = openpyxl.load_workbook(fp, read_only=True, data_only=True)
    sheets = []
    for ws in wb.worksheets:
        rows = []
        for row in ws.iter_rows(values_only=True):
            cells = [str(c) if c is not None else '' for c in row]
            if any(c.strip() for c in cells):
                rows.append(' | '.join(cells))
        if rows:
            sheets.append(f'[工作表 {ws.title}]\n' + '\n'.join(rows))
    return '\n\n'.join(sheets)

def read_pptx(fp):
    from pptx import Presentation
    prs = Presentation(fp)
    slides = []
    for i, slide in enumerate(prs.slides):
        texts = []
        for shape in slide.shapes:
            if shape.has_text_frame:
                for para in shape.text_frame.paragraphs:
                    t = ''.join(run.text for run in para.runs).strip()
                    if t:
                        texts.append(t)
        if texts:
            slides.append(f'[第{i+1}页]\n' + '\n'.join(texts))
    return '\n\n'.join(slides)

if __name__ == '__main__':
    fp = sys.argv[1]
    ext = os.path.splitext(fp)[1].lower()
    try:
        if ext in ('.docx', '.doc'):
            content = read_docx(fp)
        elif ext == '.pdf':
            content = read_pdf(fp)
        elif ext in ('.xlsx', '.xls', '.csv'):
            content = read_xlsx(fp) if ext != '.csv' else open(fp, encoding='utf-8', errors='ignore').read()
        elif ext in ('.pptx', '.ppt'):
            content = read_pptx(fp)
        else:
            content = open(fp, encoding='utf-8', errors='ignore').read()
        print(json.dumps({'ok': True, 'content': content[:15000], 'chars': len(content)}))
    except Exception as e:
        print(json.dumps({'ok': False, 'error': str(e)}))
`

let readerScriptPath = null

function getReaderScript() {
  if (readerScriptPath && fs.existsSync(readerScriptPath)) return readerScriptPath
  readerScriptPath = path.join(os.tmpdir(), 'weft_desktop_doc_reader.py')
  fs.writeFileSync(readerScriptPath, READER_PY, 'utf-8')
  return readerScriptPath
}

async function readDocumentFile(filePath) {
  const script = getReaderScript()
  return new Promise((resolve) => {
    execFile('python', [script, filePath], { timeout: 30000, encoding: 'utf-8' }, (err, stdout) => {
      if (err || !stdout?.trim()) {
        resolve({ ok: false, error: err?.message || 'no output' })
        return
      }
      try {
        resolve(JSON.parse(stdout.trim()))
      } catch {
        resolve({ ok: false, error: 'parse error' })
      }
    })
  })
}

const DOC_EXTS = ['.docx', '.doc', '.pdf', '.xlsx', '.xls', '.pptx', '.ppt', '.csv']

async function getWpsWindowTitle() {
  const scriptPath = path.join(os.tmpdir(), 'weft_desktop_wps_title.ps1')
  const script = [
    '$OutputEncoding = [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)',
    '$t = Get-Process wps,et,wpp,winword,excel,powerpnt -ErrorAction SilentlyContinue |',
    '  Where-Object { $_.MainWindowTitle -match "\\.(docx?|xlsx?|pptx?|pdf|csv)" } |',
    '  Select-Object -First 1 -ExpandProperty MainWindowTitle',
    'if ($t) { Write-Output $t }',
  ].join('\n')
  fs.writeFileSync(scriptPath, script, 'utf-8')
  try {
    const result = execSync(
      `powershell -NoProfile -ExecutionPolicy Bypass -File "${scriptPath}"`,
      { timeout: 6000, encoding: 'utf-8' },
    ).trim()
    return result || null
  } catch {
    return null
  } finally {
    try { fs.unlinkSync(scriptPath) } catch {}
  }
}

function extractFilenameBase(windowTitle) {
  const match = windowTitle.match(/^(.+?)\s*[-–—|]\s*(WPS|Microsoft|Office|Word|Excel|PowerPoint)/i)
  const raw = match ? match[1].trim() : windowTitle.split(' - ')[0].trim()
  return raw.replace(/\.(docx?|xlsx?|pptx?|pdf)$/i, '').toLowerCase()
}

async function findInRecentFiles(nameQuery) {
  const scriptPath = path.join(os.tmpdir(), 'weft_desktop_recent.ps1')
  const script = [
    '$OutputEncoding = [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)',
    '$shell = New-Object -ComObject WScript.Shell',
    '$recent = [Environment]::GetFolderPath("Recent")',
    '$lnks = Get-ChildItem $recent -Filter "*.lnk" -ErrorAction SilentlyContinue |',
    '  Sort-Object LastWriteTime -Descending | Select-Object -First 80',
    'foreach ($lnk in $lnks) {',
    '  try {',
    '    $t = $shell.CreateShortcut($lnk.FullName).TargetPath',
    '    if ($t -and (Test-Path $t)) { Write-Output $t }',
    '  } catch {}',
    '}',
  ].join('\n')
  fs.writeFileSync(scriptPath, script, 'utf-8')
  try {
    const out = execSync(
      `powershell -NoProfile -ExecutionPolicy Bypass -File "${scriptPath}"`,
      { timeout: 8000, encoding: 'utf-8' },
    ).trim()
    if (!out) return null
    const extSet = new Set(DOC_EXTS)
    const queryLower = nameQuery.toLowerCase()
    for (const line of out.split('\n').map((l) => l.trim()).filter(Boolean)) {
      const ext = path.extname(line).toLowerCase()
      if (!extSet.has(ext)) continue
      const baseFull = path.basename(line).toLowerCase()
      const baseStem = path.basename(line, ext).toLowerCase()
      if (baseFull === queryLower || baseStem.includes(queryLower) || queryLower.includes(baseStem.substring(0, 4))) {
        return line
      }
    }
  } catch {
    return null
  } finally {
    try { fs.unlinkSync(scriptPath) } catch {}
  }
  return null
}

function searchDirsForFile(filename, depth = 2) {
  const searchDirs = [
    process.cwd(),
    path.join(os.homedir(), 'Desktop'),
    path.join(os.homedir(), 'Documents'),
    'D:\\',
    'E:\\',
  ]
  const lowerName = filename.toLowerCase()

  function searchDir(dir, currentDepth) {
    try {
      const direct = path.join(dir, filename)
      if (fs.existsSync(direct)) return direct
      if (currentDepth <= 0) return null
      const entries = fs.readdirSync(dir, { withFileTypes: true })
      for (const entry of entries) {
        if (entry.isFile() && entry.name.toLowerCase() === lowerName) {
          return path.join(dir, entry.name)
        }
        if (entry.isDirectory() && currentDepth > 1) {
          const found = searchDir(path.join(dir, entry.name), currentDepth - 1)
          if (found) return found
        }
      }
    } catch {}
    return null
  }

  for (const dir of searchDirs) {
    const found = searchDir(dir, depth)
    if (found) return found
  }
  return null
}

async function findDocumentByTitle(windowTitle) {
  const procTitle = await getWpsWindowTitle().catch(() => null)
  const titleToUse = procTitle || windowTitle || ''
  if (!titleToUse) return null

  const fullNameMatch = titleToUse.match(/([^\\/:*?"<>|\r\n]+\.(docx?|xlsx?|xls|pptx?|pdf|csv))/i)
  if (fullNameMatch) {
    const filename = fullNameMatch[1]
    const fsPath = searchDirsForFile(filename, 2)
    if (fsPath) return fsPath
    const recentExact = await findInRecentFiles(filename.toLowerCase())
    if (recentExact) return recentExact
  }

  const nameBase = extractFilenameBase(titleToUse)
  if (nameBase.length >= 2) {
    const recentFuzzy = await findInRecentFiles(nameBase)
    if (recentFuzzy) return recentFuzzy
  }

  return null
}

function formatForClaude(filePath, content, maxChars = 8000) {
  const ext = path.extname(filePath).toLowerCase()
  const name = path.basename(filePath)
  const typeMap = {
    '.docx': 'Word文档',
    '.doc': 'Word文档',
    '.pdf': 'PDF',
    '.xlsx': 'Excel表格',
    '.xls': 'Excel表格',
    '.pptx': 'PPT演示文稿',
    '.ppt': 'PPT演示文稿',
    '.csv': 'CSV表格',
  }
  const typeName = typeMap[ext] || '文档'
  const truncated = content.length > maxChars
    ? content.substring(0, maxChars) + `\n\n[…已截断，完整文档共${content.length}字]`
    : content
  return `[${typeName}：${name}]\n\n${truncated}`
}

module.exports = { readDocumentFile, findDocumentByTitle, formatForClaude, DOC_EXTS }
