/**
 * 阿里云 NLS 一句话识别（RESTful API）
 * 文档：https://help.aliyun.com/zh/isi/developer-reference/restful-api-2
 *
 * 接口：POST https://nls-gateway-cn-shanghai.aliyuncs.com/stream/v1/asr
 * Header：X-NLS-Token: {token}，Content-Type: application/octet-stream
 * Query：appkey、format、sample_rate 等
 * Body：原始 PCM bytes
 */
const axios  = require('axios');
const config = require('./config');
const { getToken } = require('./token');

const ASR_URL = 'https://nls-gateway-cn-shanghai.aliyuncs.com/stream/v1/asr';

async function transcribe(pcmBuffer) {
  if (!pcmBuffer || pcmBuffer.length < 3200) {
    // 太短（< 100ms at 16kHz 16bit）→ 跳过
    return '';
  }

  const token = await getToken();

  const params = new URLSearchParams({
    appkey:                          config.aliyun.appKey,
    format:                          'pcm',
    sample_rate:                     '16000',
    enable_punctuation_prediction:   'true',
    enable_inverse_text_normalization:'true',
  });

  const resp = await axios.post(
    `${ASR_URL}?${params.toString()}`,
    pcmBuffer,
    {
      headers: {
        'X-NLS-Token':  token,
        'Content-Type': 'application/octet-stream',
      },
      timeout: 15000,
    }
  );

  const data = resp.data;
  // status 20000000 = 成功
  if (data.status !== 20000000) {
    throw new Error(`ASR 识别失败: ${data.message} (status=${data.status})`);
  }

  const text = data.result || '';
  console.log(`[ASR] 识别结果: "${text}"`);
  return text;
}

module.exports = { transcribe };
