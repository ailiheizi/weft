import { contextBridge, ipcRenderer } from 'electron';
import type { BrowserAPI, BrowserState, FindInPageResult, WorkspaceMemoryResult } from '../shared/types.js';

const browserAPI: BrowserAPI = {
  getState: () => ipcRenderer.invoke('browser:get-state'),
  createTab: (url) => ipcRenderer.invoke('browser:create-tab', url),
  activateTab: (tabId) => ipcRenderer.invoke('browser:activate-tab', tabId),
  closeTab: (tabId) => ipcRenderer.invoke('browser:close-tab', tabId),
  reopenClosedTab: () => ipcRenderer.invoke('browser:reopen-closed-tab'),
  duplicateTab: () => ipcRenderer.invoke('browser:duplicate-tab'),
  closeOtherTabs: () => ipcRenderer.invoke('browser:close-other-tabs'),
  togglePinTab: (tabId) => ipcRenderer.invoke('browser:toggle-pin-tab', tabId),
  navigate: (url) => ipcRenderer.invoke('browser:navigate', url),
  goBack: () => ipcRenderer.invoke('browser:go-back'),
  goForward: () => ipcRenderer.invoke('browser:go-forward'),
  reload: () => ipcRenderer.invoke('browser:reload'),
  stop: () => ipcRenderer.invoke('browser:stop'),
  findInPage: (query, forward) => ipcRenderer.invoke('browser:find-in-page', query, forward),
  stopFindInPage: () => ipcRenderer.invoke('browser:stop-find-in-page'),
  readPage: () => ipcRenderer.invoke('browser:read-page'),
  extractMarkdown: () => ipcRenderer.invoke('browser:extract-markdown'),
  captureScreenshot: () => ipcRenderer.invoke('browser:capture-screenshot'),
  saveMarkdown: () => ipcRenderer.invoke('browser:save-markdown'),
  getStructuredContext: () => ipcRenderer.invoke('browser:get-structured-context'),
  scanWorkspace: () => ipcRenderer.invoke('browser:scan-workspace'),
  getActionHistory: () => ipcRenderer.invoke('browser:get-action-history'),
  getWorkspaceMemory: () => ipcRenderer.invoke('browser:get-workspace-memory'),
  searchWorkspaceMemory: (query) => ipcRenderer.invoke('browser:search-workspace-memory', query),
  snapshotElements: () => ipcRenderer.invoke('browser:snapshot-elements'),
  clickRef: (ref) => ipcRenderer.invoke('browser:click-ref', ref),
  typeRef: (ref, text) => ipcRenderer.invoke('browser:type-ref', ref, text),
  actOnRef: (ref, action, text) => ipcRenderer.invoke('browser:act-on-ref', ref, action, text),
  scrollPage: (direction) => ipcRenderer.invoke('browser:scroll-page', direction),
  onFindResult: (callback) => {
    const listener = (_event: Electron.IpcRendererEvent, result: FindInPageResult) => callback(result);
    ipcRenderer.on('browser:find-result', listener);
    return () => ipcRenderer.removeListener('browser:find-result', listener);
  },
  onStateChanged: (callback) => {
    const listener = (_event: Electron.IpcRendererEvent, state: BrowserState) => callback(state);
    ipcRenderer.on('browser:state-changed', listener);
    return () => ipcRenderer.removeListener('browser:state-changed', listener);
  }
};

contextBridge.exposeInMainWorld('browserAPI', browserAPI);
