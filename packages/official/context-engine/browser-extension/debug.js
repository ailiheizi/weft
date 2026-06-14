'use strict';

const DEBUG_STORAGE_KEY = 'weft_context_debug';

async function readRecords() {
  const stored = await chrome.storage.local.get(DEBUG_STORAGE_KEY);
  return Array.isArray(stored?.[DEBUG_STORAGE_KEY]) ? stored[DEBUG_STORAGE_KEY] : [];
}

async function render() {
  const output = document.getElementById('debug-output');
  if (!output) return;
  const records = await readRecords();
  output.textContent = JSON.stringify(records, null, 2);
}

async function clearRecords() {
  await chrome.storage.local.set({ [DEBUG_STORAGE_KEY]: [] });
  await render();
}

document.getElementById('refresh-btn')?.addEventListener('click', () => {
  void render();
});

document.getElementById('clear-btn')?.addEventListener('click', () => {
  void clearRecords();
});

void render();
