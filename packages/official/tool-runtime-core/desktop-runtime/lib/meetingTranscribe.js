/**
 * 会议录音转写
 * 读取 WAV → 按 50s 分块 → 逐块调用阿里云 NLS ASR → 拼接文本
 */
const fs              = require('fs');
const { transcribe }  = require('./asr');

const SAMPLE_RATE     = 16000;
const BYTES_PER_FRAME = 2;       // int16
const CHUNK_SECONDS   = 50;      // 每块 50 秒（NLS 上限 60s，留余量）
const CHUNK_BYTES     = CHUNK_SECONDS * SAMPLE_RATE * BYTES_PER_FRAME;

/**
 * 从 WAV 文件中提取原始 PCM 数据（跳过文件头）
 */
function readWavPcm(wavPath) {
  const buf = fs.readFileSync(wavPath);

  // 查找 "data" chunk
  let offset = 12;
  while (offset < buf.length - 8) {
    const tag  = buf.toString('ascii', offset, offset + 4);
    const size = buf.readUInt32LE(offset + 4);
    if (tag === 'data') return buf.slice(offset + 8, offset + 8 + size);
    offset += 8 + (size % 2 === 0 ? size : size + 1);
  }
  // 降级：跳过标准 44 字节 WAV 头
  return buf.slice(44);
}

/**
 * 转写 WAV 文件，返回拼接文本
 * @param {string} wavPath
 * @param {(cur:number, total:number)=>void} [onProgress]
 * @returns {Promise<string>}
 */
async function transcribeWav(wavPath, onProgress) {
  const pcm    = readWavPcm(wavPath);
  const total  = Math.ceil(pcm.length / CHUNK_BYTES);
  const parts  = [];

  console.log(`[MeetingTranscribe] WAV ${Math.round(pcm.length / (SAMPLE_RATE * BYTES_PER_FRAME))}s → ${total} 块`);

  for (let i = 0; i < total; i++) {
    const chunk = pcm.slice(i * CHUNK_BYTES, (i + 1) * CHUNK_BYTES);
    if (onProgress) onProgress(i + 1, total);
    console.log(`[MeetingTranscribe] 转写 ${i + 1}/${total}…`);

    try {
      const text = await transcribe(chunk);
      if (text && text.trim()) parts.push(text.trim());
    } catch (e) {
      console.warn(`[MeetingTranscribe] 块 ${i + 1} 失败:`, e.message);
      parts.push(`[片段 ${i + 1} 转写失败]`);
    }
  }

  return parts.join('\n');
}

module.exports = { transcribeWav };
