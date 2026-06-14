const fs = require('fs')
const os = require('os')
const path = require('path')
const { execFileSync } = require('child_process')

function writableTempDir() {
  const candidates = [
    process.env.LOCALAPPDATA ? path.join(process.env.LOCALAPPDATA, 'Temp') : '',
    path.join(os.homedir(), 'AppData', 'Local', 'Temp'),
    os.tmpdir(),
    process.env.TEMP,
    process.env.TMP,
  ].filter(Boolean)
  for (const candidate of candidates) {
    try {
      fs.mkdirSync(candidate, { recursive: true })
      fs.accessSync(candidate, fs.constants.W_OK)
      return candidate
    } catch {}
  }
  return os.tmpdir()
}

function defaultPythonExe() {
  return path.join(os.homedir(), 'AppData', 'Roaming', 'weft-desktop', 'local-ocr-venv', 'Scripts', 'python.exe')
}

function captureScreen(filePath) {
  const script = `
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
$s=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$b=New-Object System.Drawing.Bitmap($s.Width,$s.Height)
$g=[System.Drawing.Graphics]::FromImage($b)
$g.CopyFromScreen($s.Location,[System.Drawing.Point]::Empty,$s.Size)
$b.Save('${filePath.replace(/\\/g, '\\\\')}',[System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose();$b.Dispose()
`.trim()
  execFileSync('powershell', ['-NoProfile', '-EncodedCommand', Buffer.from(script, 'utf16le').toString('base64')], {
    timeout: 15000,
    stdio: 'ignore',
  })
}

function runPaddleOcr(pythonExe, imagePath) {
  const code = `
import base64, json, time
from paddleocr import PaddleOCR

image_path = r'''${imagePath.replace(/'/g, "\\'")}'''
started = time.time()
ocr = PaddleOCR(use_angle_cls=False, lang='ch', show_log=False)
init_ms = int((time.time() - started) * 1000)
started = time.time()
result = ocr.ocr(image_path, cls=False)
predict_ms = int((time.time() - started) * 1000)
lines = []
for block in result or []:
    for line in block or []:
        if line and len(line) > 1:
            text = str(line[1][0]).strip()
            if text:
                lines.append(text)
payload = {"ok": True, "backend": "paddleocr-2.9.1", "init_ms": init_ms, "predict_ms": predict_ms, "line_count": len(lines), "text": "\\n".join(lines)}
print(base64.b64encode(json.dumps(payload, ensure_ascii=False).encode('utf-8')).decode('ascii'))
`.trim()
  const output = execFileSync(pythonExe, ['-c', code], {
    timeout: 90000,
    encoding: 'utf8',
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
  }).trim()
  const encoded = output.split(/\r?\n/).filter(Boolean).pop() || ''
  return JSON.parse(Buffer.from(encoded, 'base64').toString('utf8'))
}

async function main() {
  const payloadArg = process.argv[2] || ''
  const payload = payloadArg
    ? JSON.parse(payloadArg.startsWith('payload_b64:')
      ? Buffer.from(payloadArg.slice('payload_b64:'.length), 'base64').toString('utf8')
      : payloadArg)
    : {}
  const pythonExe = String(payload.python || process.env.WEFT_LOCAL_OCR_PYTHON || defaultPythonExe())
  const imagePath = path.join(writableTempDir(), `weft_screen_ocr_${Date.now()}.png`)
  try {
    captureScreen(imagePath)
    const result = runPaddleOcr(pythonExe, imagePath)
    process.stdout.write(JSON.stringify({
      ok: true,
      backend: result.backend,
      init_ms: result.init_ms,
      predict_ms: result.predict_ms,
      line_count: result.line_count,
      text: result.text,
      text_b64: Buffer.from(String(result.text || ''), 'utf8').toString('base64'),
    }))
  } finally {
    try { fs.unlinkSync(imagePath) } catch {}
  }
}

main().catch((error) => {
  process.stdout.write(JSON.stringify({ ok: false, error: error instanceof Error ? error.message : String(error) }))
  process.exit(1)
})
