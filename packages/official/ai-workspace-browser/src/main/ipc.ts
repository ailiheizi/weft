import { ipcMain } from 'electron';
import type { BrowserRuntime } from './browser-runtime.js';

export function registerBrowserIpc(runtime: BrowserRuntime): void {
  ipcMain.handle('browser:get-state', () => runtime.getState());
  ipcMain.handle('browser:get-action-history', () => runtime.getActionHistory());
  ipcMain.handle('browser:get-workspace-memory', () => runtime.getWorkspaceMemory());
  ipcMain.handle('browser:search-workspace-memory', (_event, query: string) => runtime.searchWorkspaceMemory(query));
  ipcMain.handle('browser:create-tab', (_event, url: string) => runtime.createTab(url));
  ipcMain.handle('browser:activate-tab', (_event, tabId: string) => runtime.activateTab(tabId));
  ipcMain.handle('browser:close-tab', (_event, tabId: string) => runtime.closeTab(tabId));
  ipcMain.handle('browser:reopen-closed-tab', () => runtime.reopenClosedTab());
  ipcMain.handle('browser:duplicate-tab', () => runtime.duplicateTab());
  ipcMain.handle('browser:close-other-tabs', () => runtime.closeOtherTabs());
  ipcMain.handle('browser:toggle-pin-tab', (_event, tabId?: string) => runtime.togglePinTab(tabId));
  ipcMain.handle('browser:navigate', (_event, url: string) => runtime.navigate(url));
  ipcMain.handle('browser:go-back', () => runtime.goBack());
  ipcMain.handle('browser:go-forward', () => runtime.goForward());
  ipcMain.handle('browser:reload', () => runtime.reload());
  ipcMain.handle('browser:stop', () => runtime.stop());
  ipcMain.handle('browser:find-in-page', (_event, query: string, forward?: boolean) => runtime.findInPage(query, forward));
  ipcMain.handle('browser:stop-find-in-page', () => runtime.stopFindInPage());
  ipcMain.handle('browser:read-page', () => runtime.readPage());
  ipcMain.handle('browser:extract-markdown', () => runtime.extractMarkdown());
  ipcMain.handle('browser:capture-screenshot', () => runtime.captureScreenshot());
  ipcMain.handle('browser:save-markdown', () => runtime.saveMarkdown());
  ipcMain.handle('browser:get-structured-context', () => runtime.getStructuredContext());
  ipcMain.handle('browser:scan-workspace', () => runtime.scanWorkspace());
  ipcMain.handle('browser:snapshot-elements', () => runtime.snapshotElements());
  ipcMain.handle('browser:click-ref', (_event, ref: string) => runtime.clickRef(ref));
  ipcMain.handle('browser:type-ref', (_event, ref: string, text: string) => runtime.typeRef(ref, text));
  ipcMain.handle('browser:act-on-ref', (_event, ref: string, action: 'click' | 'type' | 'select' | 'submit', text?: string) => runtime.actOnRef(ref, action, text));
  ipcMain.handle('browser:scroll-page', (_event, direction: 'up' | 'down') => runtime.scrollPage(direction));
}
