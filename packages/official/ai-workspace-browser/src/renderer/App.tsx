import React, { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import type {
  BrowserActionResult,
  BrowserState,
  ElementSnapshotResult,
  PageReadResult,
  SaveMarkdownResult,
  ScreenshotResult,
  StructuredPageContextResult,
  WorkspaceMemoryResult,
  WorkspaceScanResult
} from '../shared/types.js';

const EMPTY_STATE: BrowserState = { tabs: [], activeTabId: null, canReopenClosedTab: false };
type ToolAction = 'read' | 'markdown' | 'save' | 'screenshot' | 'structured' | 'workspace' | 'snapshot' | 'scroll-up' | 'scroll-down';

export function App(): React.ReactElement {
  const [state, setState] = useState<BrowserState>(EMPTY_STATE);
  const [address, setAddress] = useState('');
  const [status, setStatus] = useState('');
  const [selectedRef, setSelectedRef] = useState('');
  const [quickInput, setQuickInput] = useState('');
  const [toolsOpen, setToolsOpen] = useState(false);
  const [actionHistory, setActionHistory] = useState<BrowserActionResult[]>([]);
  const [workspaceMemory, setWorkspaceMemory] = useState<WorkspaceMemoryResult>({ entries: [] });
  const [memoryQuery, setMemoryQuery] = useState('');
  const [findOpen, setFindOpen] = useState(false);
  const [findQuery, setFindQuery] = useState('');
  const [findMatches, setFindMatches] = useState({ active: 0, total: 0 });
  const addressInputRef = useRef<HTMLInputElement>(null);
  const findInputRef = useRef<HTMLInputElement>(null);
  const activeTab = useMemo(() => state.tabs.find((tab) => tab.active), [state.tabs]);
  const isSecureAddress = address.startsWith('https://');
  const tabCountLabel = `${state.tabs.length} tab${state.tabs.length === 1 ? '' : 's'}`;
  const activeTabPinned = activeTab?.pinned ?? false;

  useEffect(() => {
    void window.browserAPI.getState().then(setState);
    return window.browserAPI.onStateChanged((nextState) => {
      setState(nextState);
      const nextActive = nextState.tabs.find((tab) => tab.active);
      if (nextActive) setAddress(nextActive.url);
    });
  }, []);

  useEffect(() => {
    if (activeTab) setAddress(activeTab.url);
  }, [activeTab?.id, activeTab?.url]);

  useEffect(() => window.browserAPI.onFindResult((result) => setFindMatches({ active: result.activeMatchOrdinal, total: result.matches })), []);

  useEffect(() => {
    if (findOpen) findInputRef.current?.focus();
  }, [findOpen]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent): void {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
        event.preventDefault();
        addressInputRef.current?.focus();
        addressInputRef.current?.select();
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f') {
        event.preventDefault();
        setFindOpen(true);
        return;
      }

      if (event.key === 'Escape' && findOpen) {
        event.preventDefault();
        setFindOpen(false);
        setFindQuery('');
        setFindMatches({ active: 0, total: 0 });
        void window.browserAPI.stopFindInPage();
        return;
      }

      if (event.key === 'Enter' && findOpen && findQuery.trim()) {
        event.preventDefault();
        void window.browserAPI.findInPage(findQuery, !event.shiftKey);
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 't') {
        event.preventDefault();
        void window.browserAPI.createTab('https://example.com').then(setState);
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 't') {
        event.preventDefault();
        void window.browserAPI.reopenClosedTab().then(setState);
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'w' && activeTab) {
        event.preventDefault();
        void window.browserAPI.closeTab(activeTab.id).then(setState);
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'r') {
        event.preventDefault();
        void (activeTab?.loading ? window.browserAPI.stop() : window.browserAPI.reload()).then(setState);
        return;
      }

      if (event.altKey && event.key === 'ArrowLeft') {
        event.preventDefault();
        void window.browserAPI.goBack().then(setState);
        return;
      }

      if (event.altKey && event.key === 'ArrowRight') {
        event.preventDefault();
        void window.browserAPI.goForward().then(setState);
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeTab?.id, activeTab?.loading, findOpen, findQuery]);

  async function submitAddress(event: FormEvent): Promise<void> {
    event.preventDefault();
    setState(await window.browserAPI.navigate(address));
  }

  async function reloadOrStop(): Promise<void> {
    setState(activeTab?.loading ? await window.browserAPI.stop() : await window.browserAPI.reload());
  }

  async function runTool(tool: ToolAction): Promise<void> {
    setStatus('Working');
    try {
      const result: PageReadResult | ScreenshotResult | SaveMarkdownResult | ElementSnapshotResult | StructuredPageContextResult | WorkspaceScanResult | BrowserActionResult = tool === 'read'
        ? await window.browserAPI.readPage()
        : tool === 'markdown'
          ? await window.browserAPI.extractMarkdown()
          : tool === 'save'
            ? await window.browserAPI.saveMarkdown()
            : tool === 'screenshot'
              ? await window.browserAPI.captureScreenshot()
              : tool === 'structured'
                ? await window.browserAPI.getStructuredContext()
                : tool === 'workspace'
                  ? await window.browserAPI.scanWorkspace()
                  : tool === 'snapshot'
                    ? await window.browserAPI.snapshotElements()
                    : await window.browserAPI.scrollPage(tool === 'scroll-up' ? 'up' : 'down');

      if (tool === 'snapshot') {
        const snapshot = result as ElementSnapshotResult;
        setSelectedRef(snapshot.elements[0]?.ref ?? '');
        setStatus(`Refs: ${snapshot.elements.length}`);
        return;
      }

      setStatus(summarizeResult(tool, result));
      if (tool === 'workspace') setWorkspaceMemory(await window.browserAPI.getWorkspaceMemory());
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function clickSelectedRef(): Promise<void> {
    if (!selectedRef.trim()) {
      setStatus('Run Refs, then choose @eN');
      return;
    }
    if (!window.confirm(`Click ${selectedRef}?`)) {
      setStatus(`Cancelled ${selectedRef}`);
      return;
    }
    setStatus('Clicking');
    const result = selectedRef.startsWith('@w') ? await window.browserAPI.actOnRef(selectedRef, 'click') : await window.browserAPI.clickRef(selectedRef);
    setStatus(summarizeAction(result));
    setActionHistory(await window.browserAPI.getActionHistory());
    setWorkspaceMemory(await window.browserAPI.getWorkspaceMemory());
  }

  async function submitQuickInput(): Promise<void> {
    const text = quickInput.trim();
    if (!text) return;

    if (!text.startsWith('@')) {
      if (!selectedRef.trim()) {
        setStatus('Set target ref first');
        return;
      }

      if (!window.confirm(`Type into ${selectedRef}?`)) {
        setStatus(`Cancelled ${selectedRef}`);
        return;
      }

      setStatus('Typing');
      const result = selectedRef.startsWith('@w') ? await window.browserAPI.actOnRef(selectedRef, 'type', text) : await window.browserAPI.typeRef(selectedRef, text);
      setStatus(summarizeAction(result));
      setActionHistory(await window.browserAPI.getActionHistory());
      setWorkspaceMemory(await window.browserAPI.getWorkspaceMemory());
      return;
    }

    const ref = text.split(/\s+/)[0];
    setSelectedRef(ref);
    setQuickInput('');
  }

  async function searchMemory(): Promise<void> {
    const result = await window.browserAPI.searchWorkspaceMemory(memoryQuery);
    setWorkspaceMemory(result);
    setStatus(`Memory: ${result.entries.length} matches`);
  }

  async function openMemoryEntry(url: string): Promise<void> {
    const existing = state.tabs.find((tab) => tab.url === url);
    setState(existing ? await window.browserAPI.activateTab(existing.id) : await window.browserAPI.createTab(url));
    setToolsOpen(false);
  }

  async function updateFindQuery(value: string): Promise<void> {
    setFindQuery(value);
    if (value.trim()) await window.browserAPI.findInPage(value, true);
    else {
      setFindMatches({ active: 0, total: 0 });
      await window.browserAPI.stopFindInPage();
    }
  }

  return (
    <div className="app-shell">
      <main className="browser-ui">
        <div className="tabs">
          {state.tabs.map((tab) => (
            <button
              className={tab.active ? (tab.pinned ? 'tab active pinned' : 'tab active') : tab.pinned ? 'tab pinned' : 'tab'}
              key={tab.id}
              title={`${tab.title || 'New Tab'}${tab.pinned ? ' · pinned' : ''}`}
              onClick={() => void window.browserAPI.activateTab(tab.id).then(setState)}
            >
              {tab.loading ? <span className="tab-loading" aria-label="Loading" /> : null}
              {!tab.loading ? (
                <span className="tab-favicon" aria-hidden="true">
                  {tab.favicon ? <img src={tab.favicon} alt="" /> : getSiteInitial(tab.url, tab.title)}
                </span>
              ) : null}
              <span className="tab-title">{tab.title || 'New Tab'}</span>
              <span
                className="tab-close"
                role="button"
                tabIndex={0}
                onClick={(event) => {
                  event.stopPropagation();
                  void window.browserAPI.closeTab(tab.id).then(setState);
                }}
              >
                ×
              </span>
            </button>
          ))}
          <button className="new-tab" onClick={() => void window.browserAPI.createTab('https://example.com').then(setState)}>+</button>
        </div>
        <form className="nav" onSubmit={(event) => void submitAddress(event)}>
          <button type="button" disabled={!activeTab?.canGoBack} onClick={() => void window.browserAPI.goBack().then(setState)}>←</button>
          <button type="button" disabled={!activeTab?.canGoForward} onClick={() => void window.browserAPI.goForward().then(setState)}>→</button>
          <button type="button" title={activeTab?.loading ? 'Stop' : 'Reload'} onClick={() => void reloadOrStop()}>{activeTab?.loading ? '×' : '↻'}</button>
          <label className="address-field">
            <span className={isSecureAddress ? 'origin secure' : 'origin'} title={isSecureAddress ? 'Secure connection' : 'Connection info'}>{isSecureAddress ? '◦' : '○'}</span>
            <input ref={addressInputRef} value={address} onChange={(event) => setAddress(event.target.value)} placeholder="Search or enter URL" />
          </label>
          <button type="button" title="More tools" aria-expanded={toolsOpen} onClick={() => setToolsOpen((open) => !open)}>⋯</button>
        </form>
        {findOpen ? (
          <div className="find-bar">
            <input ref={findInputRef} value={findQuery} onChange={(event) => void updateFindQuery(event.target.value)} placeholder="Find in page" />
            <span>{findMatches.total ? `${findMatches.active}/${findMatches.total}` : '0/0'}</span>
            <button type="button" onClick={() => void window.browserAPI.findInPage(findQuery, false)}>↑</button>
            <button type="button" onClick={() => void window.browserAPI.findInPage(findQuery, true)}>↓</button>
            <button type="button" onClick={() => { setFindOpen(false); setFindQuery(''); setFindMatches({ active: 0, total: 0 }); void window.browserAPI.stopFindInPage(); }}>×</button>
          </div>
        ) : null}
        {toolsOpen ? (
          <div className="tools-menu">
            <div className="menu-header">
              <span>Page tools</span>
              <span>{tabCountLabel}</span>
            </div>
            <div className="tools-row">
              <button type="button" onClick={() => void runTool('markdown')}>Extract</button>
              <button type="button" onClick={() => void runTool('save')}>Save</button>
              <button type="button" onClick={() => void runTool('snapshot')}>Refs</button>
              <button type="button" onClick={() => void runTool('structured')}>Data</button>
              <button type="button" onClick={() => void runTool('workspace')}>Observe</button>
              <button type="button" onClick={() => void runTool('screenshot')}>Shot</button>
            </div>
            {workspaceMemory.entries.length ? (
              <div className="menu-header">
                <span>Workspace memory</span>
                <span>{workspaceMemory.entries.length}</span>
              </div>
            ) : null}
            <div className="tools-row">
              <input value={memoryQuery} onChange={(event) => setMemoryQuery(event.target.value)} placeholder="Search memory" />
              <button type="button" onClick={() => void searchMemory()}>Find</button>
            </div>
            <div className="tools-row">
              <button type="button" onClick={() => void window.browserAPI.duplicateTab().then(setState)}>Duplicate tab</button>
              <button type="button" disabled={!state.canReopenClosedTab} onClick={() => void window.browserAPI.reopenClosedTab().then(setState)}>Reopen closed</button>
              <button type="button" disabled={state.tabs.length < 2} onClick={() => void window.browserAPI.closeOtherTabs().then(setState)}>Close others</button>
            </div>
            <div className="tools-row">
              <button type="button" disabled={!activeTab} onClick={() => void window.browserAPI.togglePinTab(activeTab?.id).then(setState)}>{activeTabPinned ? 'Unpin tab' : 'Pin tab'}</button>
            </div>
            <div className="tools-row">
              <input value={quickInput} onChange={(event) => setQuickInput(event.target.value)} placeholder="@e1 or text for selected ref" />
              <button type="button" onClick={() => void submitQuickInput()}>Run</button>
              <button type="button" onClick={() => void clickSelectedRef()}>Click</button>
              <button type="button" onClick={() => void runTool('scroll-up')}>↑</button>
              <button type="button" onClick={() => void runTool('scroll-down')}>↓</button>
            </div>
            {actionHistory.length ? (
              <div className="action-history">
                {actionHistory.slice(0, 3).map((action, index) => (
                  <div className="action-history-item" key={`${action.message}-${index}`}>{summarizeAction(action)}</div>
                ))}
              </div>
            ) : null}
            {workspaceMemory.entries.length ? (
              <div className="action-history">
                {workspaceMemory.entries.slice(0, 3).map((entry) => (
                  <button className="memory-item" key={entry.id} type="button" onClick={() => void openMemoryEntry(entry.url)}>{entry.type}: {entry.summary}</button>
                ))}
              </div>
            ) : null}
            {status || selectedRef ? <div className="tools-status">{selectedRef ? `${selectedRef} · ` : ''}{status}</div> : null}
          </div>
        ) : null}
      </main>
    </div>
  );
}

function getSiteInitial(url: string, title: string): string {
  try {
    const host = new URL(url).hostname.replace(/^www\./, '');
    return (host[0] ?? title[0] ?? '•').toUpperCase();
  } catch {
    return (title[0] ?? '•').toUpperCase();
  }
}

function summarizeResult(tool: ToolAction, result: PageReadResult | ScreenshotResult | SaveMarkdownResult | ElementSnapshotResult | StructuredPageContextResult | WorkspaceScanResult | BrowserActionResult): string {
  if (tool === 'read' || tool === 'markdown') return `Extracted ${(result as PageReadResult).text.length.toLocaleString()} chars`;
  if (tool === 'save') return `Saved ${(result as SaveMarkdownResult).path}`;
  if (tool === 'screenshot') return `Screenshot ${(result as ScreenshotResult).path}`;
  if (tool === 'structured') {
    const context = result as StructuredPageContextResult;
    return `${context.links.length} links, ${context.forms.length} forms, ${context.tables.length} tables`;
  }
  if (tool === 'workspace') {
    const scan = result as WorkspaceScanResult;
    const readable = scan.pages.filter((page) => page.text || page.links.length || page.forms.length).length;
    const forms = scan.pages.reduce((total, page) => total + page.forms.length, 0);
    const tables = scan.pages.reduce((total, page) => total + page.tables.length, 0);
    const links = scan.pages.reduce((total, page) => total + page.links.length, 0);
    const elements = scan.pages.reduce((total, page) => total + page.elements.filter((element) => element.interactable).length, 0);
    const axNodes = scan.pages.reduce((total, page) => total + page.accessibilityNodeCount, 0);
    const semanticAx = scan.pages.reduce((total, page) => total + page.accessibilityNodes.filter((node) => node.name).length, 0);
    const headings = scan.pages.reduce((total, page) => total + page.pageInfo.headings.length, 0);
    const domNodes = scan.pages.reduce((total, page) => total + page.pageInfo.documentNodeCount, 0);
    const resources = scan.pages.reduce((total, page) => total + page.pageInfo.resources.total, 0);
    const storage = scan.pages.reduce((total, page) => total + page.pageInfo.storage.cookies + page.pageInfo.storage.localStorageKeys + page.pageInfo.storage.sessionStorageKeys, 0);
    const issues = scan.pages.reduce((total, page) => total + page.consoleMessages.length + page.networkIssues.length, 0);
    const loaded = scan.pages.filter((page) => page.pageInfo.performance.loadMs > 0).length;
    return `Observes ${readable}/${scan.pages.length} tabs · ${elements} actions · ${links} links · ${forms} forms · ${tables} tables · ${headings} headings · ${domNodes} DOM · ${resources} resources · ${storage} storage · ${issues} issues · ${loaded} timings · ${semanticAx}/${axNodes} AX`;
  }
  return (result as BrowserActionResult).message;
}

function summarizeAction(result: BrowserActionResult): string {
  if (typeof result.changed === 'boolean') {
    const observed = typeof result.observedActions === 'number' ? ` · sees ${result.observedActions} actions` : '';
    return `${result.message} · ${result.changed ? 'changed' : 'verified no page change'}${observed}`;
  }
  return result.message;
}
