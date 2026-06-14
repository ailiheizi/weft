#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const http = require('http');

const POLL_MS = Number(process.env.BLINK_POLL_MS || 5000);
const COOLDOWN_MS = Number(process.env.BLINK_COOLDOWN_MS || 120000);
const MIN_STABLE_MS = Number(process.env.BLINK_MIN_STABLE_MS || 30000);
const EXIT_AFTER_EMIT = process.env.BLINK_EXIT_AFTER_EMIT === '1';
const PORT = Number(process.env.WEFT_CONTEXT_ENGINE_PORT || 43131);
const STATUS_FILE = path.join(
  process.env.USERPROFILE || 'C:\\Users\\Administrator',
  '.openclaw', 'workspace', 'pc-status.json'
);

const BLOCKED_STATUS = new Set(['meeting', 'gaming', 'away', 'idle']);
const BLOCKED_PROCESS_RE = /(zoom|tencentmeeting|wechat|wecom|teams|slack|discord|obs|game|dota|lol|valorant|cs2|steam)/i;
const PREFERRED_PROCESS_RE = /(chrome|msedge|firefox|brave|code|cursor|windowsterminal|powershell|pwsh|cmd|notepad|word|excel|powerpnt|wps|typora|obsidian)/i;

let lastWindowKey = '';
let stableSince = 0;
let lastTriggeredAt = 0;
let timer = null;

function readStatus() {
  try {
    const raw = fs.readFileSync(STATUS_FILE, 'utf8');
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function trimString(value) {
  return typeof value === 'string' ? value.trim() : '';
}

function normalizeContext(ctx) {
  if (!ctx || typeof ctx !== 'object') return null;
  const detail = ctx.detail && typeof ctx.detail === 'object' ? ctx.detail : {};
  const status = trimString(ctx.status).toLowerCase();
  const processName = trimString(detail.fg_process || ctx.foreground).toLowerCase();
  const title = trimString(detail.fg_window || ctx.fg_title);
  return {
    status,
    processName,
    title,
    updatedAt: trimString(ctx.updated_at),
  };
}

function shouldTriggerBlink(ctx) {
  if (!ctx) return { allow: false, reason: 'no-context' };
  if (!ctx.title || !ctx.processName) return { allow: false, reason: 'missing-surface' };
  if (BLOCKED_STATUS.has(ctx.status)) return { allow: false, reason: `blocked-status:${ctx.status}` };
  if (BLOCKED_PROCESS_RE.test(ctx.processName)) return { allow: false, reason: `blocked-process:${ctx.processName}` };
  if (!PREFERRED_PROCESS_RE.test(ctx.processName)) return { allow: false, reason: `non-focus-process:${ctx.processName}` };
  return { allow: true, reason: 'stable-focus-window' };
}

function postSkillEvent(eventType, payload) {
  return new Promise((resolve, reject) => {
    const body = JSON.stringify({
      action: 'ingest_external_event',
      data: {
        event_type: eventType,
        payload,
      },
    });
    const req = http.request({
      host: '127.0.0.1',
      port: PORT,
      path: '/webhook',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(body),
      },
    }, (res) => {
      let raw = '';
      res.setEncoding('utf8');
      res.on('data', chunk => { raw += chunk; });
      res.on('end', () => {
        if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
          resolve(raw);
          return;
        }
        reject(new Error(`HTTP ${res.statusCode || 0} ${raw}`.trim()));
      });
    });
    req.on('error', reject);
    req.write(body);
    req.end();
  });
}

async function tick() {
  const rawCtx = readStatus();
  const ctx = normalizeContext(rawCtx);
  if (!ctx) return;

  const windowKey = `${ctx.processName}::${ctx.title}`;
  const now = Date.now();

  if (windowKey !== lastWindowKey) {
    lastWindowKey = windowKey;
    stableSince = now;
    return;
  }

  if ((now - stableSince) < MIN_STABLE_MS) return;
  if ((now - lastTriggeredAt) < COOLDOWN_MS) return;

  const decision = shouldTriggerBlink(ctx);
  if (!decision.allow) return;

  const payload = {
    reason: decision.reason,
    window_title: ctx.title,
    process: ctx.processName,
    status: ctx.status || 'active',
    stable_ms: now - stableSince,
    observed_at: new Date().toISOString(),
    producer: 'blink-producer',
  };

  try {
    await postSkillEvent('blink_signal_detected', payload);
    lastTriggeredAt = now;
    if (EXIT_AFTER_EMIT) {
      if (timer) clearInterval(timer);
      process.exit(0);
    }
  } catch {}
}

timer = setInterval(() => {
  void tick();
}, POLL_MS);
void tick();
