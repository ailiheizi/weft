/**
 * 阿里云 NLS Token 管理
 *
 * 优先使用 AccessKey 自动获取 Token（有效期约 12 小时，自动刷新）
 * 若未配置 AccessKey，则使用 .env 里的静态 Token
 */
const axios  = require('axios');
const crypto = require('crypto');
const config = require('./config');

let _cachedToken    = null;
let _tokenExpireAt  = 0;    // Unix timestamp（秒）

/**
 * 用 AccessKey 调用阿里云 CreateToken API
 * 参考：https://help.aliyun.com/zh/isi/getting-started/obtain-an-access-token
 */
async function createTokenViaAK() {
  const { akId, akSecret } = config.aliyun;
  if (!akId || !akSecret) throw new Error('未配置 ALIYUN_AK_ID / ALIYUN_AK_SECRET');

  // 构造签名参数
  const params = {
    AccessKeyId:      akId,
    Action:           'CreateToken',
    Format:           'JSON',
    RegionId:         'cn-shanghai',
    SignatureMethod:  'HMAC-SHA1',
    SignatureNonce:   Math.random().toString(36).slice(2),
    SignatureVersion: '1.0',
    Timestamp:        new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
    Version:          '2019-02-28',
  };

  // 按 key 排序拼接
  const sortedKeys = Object.keys(params).sort();
  const canonicalQuery = sortedKeys
    .map(k => `${encodeURIComponent(k)}=${encodeURIComponent(params[k])}`)
    .join('&');

  const stringToSign =
    'GET&' +
    encodeURIComponent('/') + '&' +
    encodeURIComponent(canonicalQuery);

  const signature = crypto
    .createHmac('sha1', akSecret + '&')
    .update(stringToSign)
    .digest('base64');

  const url =
    'http://nls-meta.cn-shanghai.aliyuncs.com/?' +
    canonicalQuery +
    '&Signature=' + encodeURIComponent(signature);

  const resp = await axios.get(url, { timeout: 10000 });
  const token      = resp.data.Token?.Id;
  const expireTime = resp.data.Token?.ExpireTime;  // Unix 秒

  if (!token) throw new Error('CreateToken 失败：' + JSON.stringify(resp.data));

  console.log(`[Token] 获取成功，过期时间: ${new Date(expireTime * 1000).toLocaleString()}`);
  return { token, expireTime };
}

/**
 * 获取有效 Token（优先使用 AK 自动刷新，其次使用静态 Token）
 */
async function getToken() {
  const now = Math.floor(Date.now() / 1000);

  // 缓存未过期（提前 5 分钟刷新）
  if (_cachedToken && _tokenExpireAt > now + 300) {
    return _cachedToken;
  }

  // 有 AccessKey → 自动获取（失败则降级为静态 Token）
  if (config.aliyun.akId && config.aliyun.akSecret) {
    try {
      const { token, expireTime } = await createTokenViaAK();
      _cachedToken   = token;
      _tokenExpireAt = expireTime;
      return token;
    } catch (e) {
      console.warn('[Token] AK 获取失败，降级使用静态 Token:', e.message);
    }
  }

  // 用静态 Token（降级方案）
  const staticToken = config.aliyun.token;
  if (staticToken) {
    console.log('[Token] 使用静态 Token（注意 12 小时过期）');
    _cachedToken   = staticToken;
    _tokenExpireAt = now + 11 * 3600; // 假设 11 小时有效
    return staticToken;
  }

  throw new Error(
    '未配置 Token！请在 .env 中填写 ALIYUN_AK_ID+ALIYUN_AK_SECRET 或 ALIYUN_NLS_TOKEN'
  );
}

module.exports = { getToken };
