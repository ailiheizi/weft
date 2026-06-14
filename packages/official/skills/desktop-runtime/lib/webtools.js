/**
 * Desktop web fetch helper.
 */
'use strict'

const axios = require('axios')

const MAX_CONTENT = 80000
const CACHE_TTL = 15 * 60 * 1000
const cache = new Map()

let td = null
function getTD() {
  if (td) return td
  const TurndownService = require('turndown')
  td = new TurndownService({ headingStyle: 'atx', codeBlockStyle: 'fenced', bulletListMarker: '-' })
  td.remove(['script', 'style', 'noscript', 'iframe', 'nav', 'header', 'footer', 'aside', 'svg', 'img'])
  return td
}

const HTTP = axios.create({
  timeout: 15000,
  maxContentLength: 5 * 1024 * 1024,
  headers: {
    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36',
    'Accept-Language': 'zh-CN,zh;q=0.9,en;q=0.8',
    Accept: 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
  },
})

async function fetchUrl(url) {
  const cached = cache.get(url)
  if (cached && Date.now() - cached.ts < CACHE_TTL) return cached.content

  const resp = await HTTP.get(url, { responseType: 'text' })
  const ct = resp.headers['content-type'] || ''
  let content = ''

  if (ct.includes('text/html')) {
    try {
      content = getTD().turndown(resp.data)
    } catch {
      content = resp.data.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim()
    }
  } else if (ct.includes('text/') || ct.includes('json') || ct.includes('xml')) {
    content = typeof resp.data === 'string' ? resp.data : JSON.stringify(resp.data, null, 2)
  } else {
    return `[无法处理的内容类型: ${ct}]`
  }

  content = content.replace(/\n{4,}/g, '\n\n').trim()
  if (content.length > MAX_CONTENT) {
    content = content.substring(0, MAX_CONTENT) + `\n\n[内容已截断，原始长度 ${content.length} 字符]`
  }

  cache.set(url, { content, ts: Date.now() })
  return content
}

module.exports = { fetchUrl }
