/**
 * 会议录音管理器
 * 启动 / 停止 Python 录音进程，返回 WAV 文件路径
 */
const { spawn } = require('child_process');
const path      = require('path');
const os        = require('os');
const fs        = require('fs');

const PYTHON     = process.env.PYTHON_PATH
  || 'C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python311\\python.exe';
const SCRIPT     = path.join(__dirname, 'meeting_recorder.py');
const WORK_DIR   = process.env.DEEPSEEK_WORK_DIR || path.join(os.homedir(), 'Desktop');

let _proc      = null;
let _outPath   = null;
let _startedAt = null;

/** 当前是否正在录音（进程存活） */
function isRecording() {
  return !!_proc && _proc.exitCode === null;
}

/** 录音是否已启动过（进程已创建，无论是否仍在运行） */
function hasActiveSession() {
  return !!_outPath;
}

/** 录音已进行的秒数 */
function elapsedSeconds() {
  if (!_startedAt) return 0;
  return Math.round((Date.now() - _startedAt) / 1000);
}

/**
 * 开始录音
 * @returns {Promise<string>} WAV 文件路径
 */
async function startRecording() {
  if (isRecording()) throw new Error('已在录音中，请先停止当前录音');
  if (_outPath) {
    // 上次会话残留，清理状态
    _proc = null; _outPath = null; _startedAt = null;
  }

  fs.mkdirSync(WORK_DIR, { recursive: true });
  const ts   = new Date().toISOString().replace(/[:.]/g, '-').substring(0, 16);
  _outPath   = path.join(WORK_DIR, `会议录音_${ts}.wav`);
  _startedAt = Date.now();

  _proc = spawn(PYTHON, [SCRIPT, '--output', _outPath], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });

  // 捕获局部引用，避免 _proc 被 stopRecording() 清零后事件 handler 崩溃
  const proc = _proc;
  proc.stderr.on('data', d => console.warn('[Recorder/err]', d.toString().trim()));
  proc.stdout.on('data', d => console.log('[Recorder/out]', d.toString().trim()));
  proc.on('error', e => console.warn('[Recorder] 进程错误:', e.message));

  // 等待录音器确认启动（最多 12 秒）
  await new Promise((resolve, reject) => {
    let settled = false;
    const settle = (fn, arg) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      proc.stdout.off('data', onData);
      fn(arg);
    };

    const timer = setTimeout(
      () => settle(reject, new Error('录音器启动超时，请检查 sounddevice 是否正常')),
      12000,
    );

    const onData = chunk => {
      const line = chunk.toString();
      if (line.includes('START')) settle(resolve, undefined);
      if (line.includes('ERROR')) settle(reject, new Error(line.trim()));
    };

    proc.stdout.on('data', onData);

    proc.on('exit', code => {
      if (code !== 0) settle(reject, new Error(`录音器异常退出 (code ${code})`));
      else settle(resolve, undefined);
    });
  });

  console.log(`[Recorder] ▶ 开始录音: ${_outPath}`);
  return _outPath;
}

/**
 * 停止录音
 * @returns {Promise<{path: string, duration: number}>}
 */
async function stopRecording() {
  // 即使进程已崩溃，只要有路径就允许"停止"（清理状态）
  if (!_outPath) throw new Error('当前没有进行中的录音');

  const outPath  = _outPath;
  const duration = elapsedSeconds();
  const proc     = _proc;

  // 清理状态，让后续调用不再报"当前没有录音"而是可以重新开始
  _proc = null; _outPath = null; _startedAt = null;

  if (proc && proc.exitCode === null) {
    // 进程仍在运行：发送 stop 命令，等待退出
    try { proc.stdin.write('stop\n'); } catch {}

    await new Promise(resolve => {
      const timer = setTimeout(() => {
        try { proc.kill('SIGTERM'); } catch {}
        setTimeout(resolve, 1000);
      }, 10000);
      proc.on('exit', () => { clearTimeout(timer); resolve(); });
    });
  }
  // 若进程已退出（崩溃），直接检查文件

  if (!fs.existsSync(outPath)) {
    throw new Error('录音文件未生成，可能录音时间太短或录音器已崩溃');
  }
  const stat = fs.statSync(outPath);
  if (stat.size < 1000) throw new Error('录音文件过小，请检查音频设备权限');

  console.log(`[Recorder] ■ 停止录音: ${outPath} (${duration}s, ${Math.round(stat.size / 1024)}KB)`);
  return { path: outPath, duration };
}

/** 强制清理，不保存文件（用于错误恢复） */
function forceReset() {
  if (_proc && _proc.exitCode === null) {
    try { _proc.kill('SIGTERM'); } catch {}
  }
  _proc = null; _outPath = null; _startedAt = null;
}

module.exports = { isRecording, hasActiveSession, elapsedSeconds, startRecording, stopRecording, forceReset };
