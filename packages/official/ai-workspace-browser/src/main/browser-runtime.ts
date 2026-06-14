import { app, BaseWindow, WebContents, WebContentsView } from 'electron';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { extractMarkdownFromHtml } from './extractors.js';
import type {
  BrowserActionResult,
  BrowserState,
  BrowserTabState,
  ElementSnapshotResult,
  PageReadResult,
  SaveMarkdownResult,
  ScreenshotResult,
  StructuredPageContextResult,
  AccessibilityNodeSummary,
  WorkspaceMemoryEntry,
  WorkspaceMemoryResult,
  WorkspaceScanResult
} from '../shared/types.js';

interface RuntimeTab {
  id: string;
  view: WebContentsView;
  favicon: string;
  pinned: boolean;
  consoleMessages: Array<{ level: string; text: string; line: number; sourceId: string }>;
  networkIssues: Array<{ url: string; error: string }>;
  networkRequests: Array<{ id: string; url: string; method: string; type: string; status: number | null; fromCache: boolean; startedAt: string; endedAt: string | null }>;
  dialogs: Array<{ kind: string; message: string; createdAt: string }>;
  popups: Array<{ url: string; frameName: string; disposition: string; createdAt: string }>;
  downloads: Array<{ url: string; filename: string; state: string; createdAt: string }>;
  recentEvents: Array<{ type: string; detail: string; url: string; createdAt: string }>;
}

interface PersistedSession {
  tabs: Array<{ url: string; pinned: boolean }>;
  activeIndex: number;
}

interface PersistedMemory {
  entries: WorkspaceMemoryEntry[];
}

const UI_TOP_HEIGHT = 64;
const DEFAULT_URL = 'https://example.com';
const SESSION_FILE = 'ai-workspace-browser-session.json';

export class BrowserRuntime {
  private tabs = new Map<string, RuntimeTab>();
  private recentlyClosedTabs: Array<{ url: string; pinned: boolean }> = [];
  private activeTabId: string | null = null;
  private saveSessionTimer: ReturnType<typeof setTimeout> | null = null;
  private actionHistory: BrowserActionResult[] = [];
  private workspaceMemory: WorkspaceMemoryEntry[] = [];
  private lastFindQuery = '';
  private activeFindRequestId = 0;

  constructor(private readonly window: BaseWindow, private readonly eventTarget: WebContents) {
    this.window.on('resize', () => this.layoutActiveView());
  }

  async initialize(): Promise<void> {
    await this.loadMemory();
    const session = await this.loadSession();
    const restoredTabs = session?.tabs.length ? session.tabs : [{ url: DEFAULT_URL, pinned: false }];

    for (const tab of restoredTabs.slice(0, 12)) {
      await this.createTab(tab.url, tab.pinned);
    }

    const tabs = this.orderedTabs();
    const active = tabs[Math.min(Math.max(session?.activeIndex ?? 0, 0), tabs.length - 1)];
    if (active) this.activateTab(active.id);
  }

  getState(): BrowserState {
    const tabs = this.orderedTabs().map((tab) => this.toTabState(tab));
    return { tabs, activeTabId: this.activeTabId, canReopenClosedTab: this.recentlyClosedTabs.length > 0 };
  }

  getActionHistory(): BrowserActionResult[] {
    return this.actionHistory;
  }

  getWorkspaceMemory(): WorkspaceMemoryResult {
    return { entries: this.workspaceMemory };
  }

  searchWorkspaceMemory(query: string): WorkspaceMemoryResult {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return this.getWorkspaceMemory();
    return {
      entries: this.workspaceMemory.filter((entry) => `${entry.title} ${entry.url} ${entry.summary} ${entry.type}`.toLowerCase().includes(normalized))
    };
  }

  async createTab(url: string, pinned = false): Promise<BrowserState> {
    const id = randomUUID();
    const view = new WebContentsView({
      webPreferences: {
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: true,
        partition: 'persist:ai-workspace-browser'
      }
    });

    const tab: RuntimeTab = { id, view, favicon: '', pinned, consoleMessages: [], networkIssues: [], networkRequests: [], dialogs: [], popups: [], downloads: [], recentEvents: [] };
    this.tabs.set(id, tab);
    this.attachTabEvents(tab);
    this.window.contentView.addChildView(view);
    await view.webContents.loadURL(normalizeUrl(url));
    this.activateTab(id);
    return this.getState();
  }

  async runFixtureSelfTest(): Promise<void> {
    const fixtureUrl = createFixtureDataUrl();
    await this.createTab(fixtureUrl);
    this.closeOtherTabs();

    const read = await this.readPage();
    if (!read.text.includes('Fixture Page')) throw new Error('Fixture readPage did not include heading text');

    const structured = await this.getStructuredContext();
    if (!structured.links.length || !structured.forms.length || !structured.tables.length) {
      throw new Error('Fixture structured context missing links/forms/tables');
    }

    const workspace = await this.scanWorkspace();
    const page = workspace.pages[0];
    if (!page) throw new Error('Fixture workspace scan returned no pages');
    const typeTarget = page.elements.find((element) => element.actions.includes('type'));
    const clickTarget = page.elements.find((element) => element.actions.includes('click') && element.name.includes('Apply'));
    if (!typeTarget || !clickTarget) {
      throw new Error(`Fixture observe did not expose type/click targets: url=${page.url} title=${page.title} text=${page.text.slice(0, 200)} error=${page.error ?? ''} elements=${page.elements.map((element) => `${element.ref}:${element.role}:${element.name}:${element.actions.join('|')}`).join(', ')}`);
    }

    await this.actOnRef(typeTarget.ref, 'type', 'hello');
    await this.actOnRef(clickTarget.ref, 'click');

    const updated = await this.readPage();
    if (!updated.text.includes('Clicked:hello')) throw new Error('Fixture action path did not update page text');

    this.requireActiveTab().view.webContents.focus();
    this.findInPage('Clicked');
    await wait(100);
    this.stopFindInPage();
  }

  activateTab(tabId: string): BrowserState {
    const tab = this.requireTab(tabId);
    this.activeTabId = tab.id;

    for (const candidate of this.tabs.values()) {
      candidate.view.setVisible(candidate.id === tab.id);
    }

    this.layoutActiveView();
    this.emitState();
    return this.getState();
  }

  closeTab(tabId: string): BrowserState {
    const tab = this.requireTab(tabId);
    const url = tab.view.webContents.getURL();
    if (url) this.recentlyClosedTabs.unshift({ url, pinned: tab.pinned });
    this.recentlyClosedTabs = this.recentlyClosedTabs.slice(0, 10);
    this.window.contentView.removeChildView(tab.view);
    tab.view.webContents.close();
    this.tabs.delete(tabId);

    if (this.activeTabId === tabId) {
      const next = this.orderedTabs()[0];
      this.activeTabId = next?.id ?? null;
    }

    if (this.activeTabId) {
      this.activateTab(this.activeTabId);
    } else {
      void this.createTab(DEFAULT_URL);
    }

    this.emitState();
    return this.getState();
  }

  async reopenClosedTab(): Promise<BrowserState> {
    const tab = this.recentlyClosedTabs.shift();
    if (!tab) return this.getState();
    return this.createTab(tab.url, tab.pinned);
  }

  async duplicateTab(): Promise<BrowserState> {
    return this.createTab(this.requireActiveTab().view.webContents.getURL() || DEFAULT_URL);
  }

  closeOtherTabs(): BrowserState {
    const activeTab = this.requireActiveTab();
    for (const tab of Array.from(this.tabs.values())) {
      if (tab.id === activeTab.id) continue;
      this.window.contentView.removeChildView(tab.view);
      tab.view.webContents.close();
      this.tabs.delete(tab.id);
    }
    this.activateTab(activeTab.id);
    return this.getState();
  }

  togglePinTab(tabId?: string): BrowserState {
    const tab = this.requireTab(tabId ?? this.requireActiveTab().id);
    tab.pinned = !tab.pinned;
    this.emitState();
    return this.getState();
  }

  async navigate(url: string): Promise<BrowserState> {
    const tab = this.requireActiveTab();
    await tab.view.webContents.loadURL(normalizeUrl(url));
    this.emitState();
    return this.getState();
  }

  goBack(): BrowserState {
    const webContents = this.requireActiveTab().view.webContents;
    if (webContents.canGoBack()) webContents.goBack();
    this.emitState();
    return this.getState();
  }

  goForward(): BrowserState {
    const webContents = this.requireActiveTab().view.webContents;
    if (webContents.canGoForward()) webContents.goForward();
    this.emitState();
    return this.getState();
  }

  reload(): BrowserState {
    this.requireActiveTab().view.webContents.reload();
    this.emitState();
    return this.getState();
  }

  stop(): BrowserState {
    this.requireActiveTab().view.webContents.stop();
    this.emitState();
    return this.getState();
  }

  findInPage(query: string, forward = true): void {
    const trimmed = query.trim();
    if (!trimmed) {
      this.stopFindInPage();
      return;
    }
    const findNext = trimmed === this.lastFindQuery;
    this.lastFindQuery = trimmed;
    this.activeFindRequestId = this.requireActiveTab().view.webContents.findInPage(trimmed, { forward, findNext });
  }

  stopFindInPage(): void {
    this.lastFindQuery = '';
    this.activeFindRequestId = 0;
    this.requireActiveTab().view.webContents.stopFindInPage('clearSelection');
  }

  dispose(): void {
    if (this.saveSessionTimer) {
      clearTimeout(this.saveSessionTimer);
      this.saveSessionTimer = null;
    }
    for (const tab of this.tabs.values()) {
      if (tab.view.webContents.debugger.isAttached()) tab.view.webContents.debugger.detach();
      if (!tab.view.webContents.isDestroyed()) tab.view.webContents.close();
    }
    this.tabs.clear();
    this.activeTabId = null;
  }

  async readPage(): Promise<PageReadResult> {
    return this.executePageReadScript(`
      (() => ({
        url: location.href,
        title: document.title,
        text: document.body?.innerText?.replace(/\\s+/g, ' ').trim().slice(0, 12000) ?? ''
      }))()
    `);
  }

  async extractMarkdown(): Promise<PageReadResult> {
    const page = await this.executePageReadScript(`
      (() => ({
        url: location.href,
        title: document.title,
        text: document.documentElement.outerHTML
      }))()
    `);
    const extracted = extractMarkdownFromHtml(page.text, page.url, page.title);
    return { url: page.url, title: extracted.title, text: extracted.text };
  }

  async captureScreenshot(): Promise<ScreenshotResult> {
    const image = await this.requireActiveTab().view.webContents.capturePage();
    const filePath = join(app.getPath('temp'), `ai-workspace-browser-${Date.now()}.png`);
    await writeFile(filePath, image.toPNG());
    return { path: filePath };
  }

  async saveMarkdown(): Promise<SaveMarkdownResult> {
    const page = await this.extractMarkdown();
    const directory = join(app.getPath('documents'), 'AI Workspace Browser');
    await mkdir(directory, { recursive: true });
    const fileName = `${safeFileName(page.title || 'untitled-page')}-${Date.now()}.md`;
    const filePath = join(directory, fileName);
    const content = [
      '---',
      `title: ${JSON.stringify(page.title)}`,
      `url: ${JSON.stringify(page.url)}`,
      `savedAt: ${JSON.stringify(new Date().toISOString())}`,
      '---',
      '',
      `# ${page.title || 'Untitled Page'}`,
      '',
      page.text
    ].join('\n');
    await writeFile(filePath, content, 'utf8');
    return { path: filePath, content };
  }

  async getStructuredContext(): Promise<StructuredPageContextResult> {
    const result: unknown = await this.requireActiveTab().view.webContents.executeJavaScript(STRUCTURED_CONTEXT_SCRIPT, false);
    if (!isStructuredPageContextResult(result)) {
      throw new Error('Structured context script returned invalid result');
    }
    return result;
  }

  async scanWorkspace(): Promise<WorkspaceScanResult> {
    const pages = [];
    for (const tab of Array.from(this.tabs.values())) {
      try {
        const observed = await this.observeTab(tab);
        const page = { ...observed, tabId: tab.id };
        pages.push(page);
        this.recordMemory('observation', tab.id, page.url, page.title, `${page.elements.filter((element) => element.interactable).length} actions, ${page.links.length} links, ${page.forms.length} forms`);
      } catch (error) {
        pages.push({
          tabId: tab.id,
          url: tab.view.webContents.getURL(),
          title: tab.view.webContents.getTitle() || tab.view.webContents.getURL() || 'Untitled',
          text: '',
          links: [],
          forms: [],
          tables: [],
          viewport: { width: 0, height: 0, scrollX: 0, scrollY: 0 },
          pageInfo: {
            language: '',
            readyState: '',
            origin: '',
            protocol: '',
            secureContext: false,
            description: '',
            canonicalUrl: '',
            documentNodeCount: 0,
            shadowRootCount: 0,
            textLength: 0,
            scrollable: false,
            documentSize: { width: 0, height: 0 },
            focusedElement: null,
            activeElement: null,
            selectionText: '',
            headings: [],
            landmarks: [],
            media: { images: 0, videos: 0, audios: 0, canvases: 0, iframes: 0 },
            resources: { scripts: 0, stylesheets: 0, fonts: 0, fetches: 0, total: 0 },
            storage: { cookies: 0, localStorageKeys: 0, sessionStorageKeys: 0, localStorageSampleKeys: [], sessionStorageSampleKeys: [] },
            performance: { loadMs: 0, domContentLoadedMs: 0, firstPaintMs: 0, firstContentfulPaintMs: 0, resourceDurationMs: 0 },
            lifecycle: { visibilityState: '', hidden: false, prerendering: false },
            frames: [],
            resourceSamples: [],
            browserState: { loading: tab.view.webContents.isLoading(), crashed: tab.view.webContents.isCrashed(), pendingNetworkRequests: tab.networkRequests.filter((request) => request.endedAt === null).length, permissions: {}, cookieNames: [], localStorageSamples: [], sessionStorageSamples: [] },
          },
          consoleMessages: tab.consoleMessages,
          networkIssues: tab.networkIssues,
          networkRequests: tab.networkRequests,
          dialogs: tab.dialogs,
          popups: tab.popups,
          downloads: tab.downloads,
          recentEvents: tab.recentEvents,
          accessibilityNodeCount: 0,
          accessibilityNodes: [],
          domSnapshot: { tree: '', xpathMap: {}, urlMap: {} },
          candidates: [],
          elements: [],
          error: error instanceof Error ? error.message : String(error)
        });
      }
    }
    return { pages };
  }

  async snapshotElements(): Promise<ElementSnapshotResult> {
    const result: unknown = await this.requireActiveTab().view.webContents.executeJavaScript(SNAPSHOT_SCRIPT, false);
    if (!isElementSnapshotResult(result)) {
      throw new Error('Snapshot script returned invalid result');
    }
    return result;
  }

  async clickRef(ref: string): Promise<BrowserActionResult> {
    return this.executeActionScript(`
      (() => {
        const element = window.__aiWorkspaceRefs?.[${JSON.stringify(ref)}];
        if (!element) return { ok: false, message: 'Unknown ref. Run Snapshot first.' };
        element.scrollIntoView({ block: 'center', inline: 'center' });
        element.click();
        return { ok: true, message: 'Clicked ${escapeForTemplate(ref)}' };
      })()
    `);
  }

  async typeRef(ref: string, text: string): Promise<BrowserActionResult> {
    return this.executeActionScript(`
      (() => {
        const element = window.__aiWorkspaceRefs?.[${JSON.stringify(ref)}];
        if (!element) return { ok: false, message: 'Unknown ref. Run Snapshot first.' };
        if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element.isContentEditable)) {
          return { ok: false, message: 'Ref is not a text input' };
        }
        element.scrollIntoView({ block: 'center', inline: 'center' });
        element.focus();
        if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
          element.value = ${JSON.stringify(text)};
          element.dispatchEvent(new Event('input', { bubbles: true }));
          element.dispatchEvent(new Event('change', { bubbles: true }));
        } else {
          element.textContent = ${JSON.stringify(text)};
          element.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: ${JSON.stringify(text)} }));
        }
        return { ok: true, message: 'Typed into ${escapeForTemplate(ref)}' };
      })()
    `);
  }

  async actOnRef(ref: string, action: 'click' | 'type' | 'select' | 'submit', text = ''): Promise<BrowserActionResult> {
    const { tab, localRef } = this.resolveObservedRef(ref);
    const beforeUrl = tab.view.webContents.getURL();
    const beforeTitle = tab.view.webContents.getTitle();
    const precheck = await this.executeActionScriptInTab(tab, `
      (() => {
        const element = window.__aiWorkspaceRefs?.[${JSON.stringify(localRef)}];
        if (!element) return { ok: false, message: 'Unknown observed ref. Run Observe first.' };
        const rect = element.getBoundingClientRect();
        const visible = rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.right >= 0 && rect.top <= window.innerHeight && rect.left <= window.innerWidth;
        if (!visible) return { ok: false, message: 'Observed ref is no longer visible' };
        if (element.disabled || element.getAttribute('aria-disabled') === 'true') return { ok: false, message: 'Observed ref is disabled' };
        return { ok: true, message: 'Observed ref revalidated' };
      })()
    `);
    if (!precheck.ok) {
      const failed = { ...precheck, action, ref, error: precheck.message, revalidated: false, revalidationError: precheck.message, beforeUrl, beforeTitle, afterUrl: beforeUrl, afterTitle: beforeTitle, changed: false };
      this.actionHistory.unshift(failed);
      this.actionHistory = this.actionHistory.slice(0, 20);
      return failed;
    }
    const result = await this.executeActionScriptInTab(tab, `
      (() => {
        const element = window.__aiWorkspaceRefs?.[${JSON.stringify(localRef)}];
        if (!element) return { ok: false, message: 'Unknown observed ref. Run Observe first.' };
        element.scrollIntoView({ block: 'center', inline: 'center' });
        element.focus?.();
        if (${JSON.stringify(action)} === 'click') {
          element.click();
          return { ok: true, message: 'Clicked ${escapeForTemplate(ref)}' };
        }
        if (${JSON.stringify(action)} === 'submit') {
          const form = element instanceof HTMLFormElement ? element : element.closest('form');
          if (!form) return { ok: false, message: 'No form for ${escapeForTemplate(ref)}' };
          form.requestSubmit?.();
          return { ok: true, message: 'Submitted ${escapeForTemplate(ref)}' };
        }
        if (${JSON.stringify(action)} === 'select') {
          if (!(element instanceof HTMLSelectElement)) return { ok: false, message: 'Ref is not a select' };
          const option = Array.from(element.options).find((candidate) => candidate.value === ${JSON.stringify(text)} || candidate.text === ${JSON.stringify(text)});
          if (!option) return { ok: false, message: 'No matching option' };
          element.value = option.value;
          element.dispatchEvent(new Event('input', { bubbles: true }));
          element.dispatchEvent(new Event('change', { bubbles: true }));
          return { ok: true, message: 'Selected ${escapeForTemplate(ref)}' };
        }
        if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element.isContentEditable)) {
          return { ok: false, message: 'Ref is not text-editable' };
        }
        if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
          element.value = ${JSON.stringify(text)};
          element.dispatchEvent(new Event('input', { bubbles: true }));
          element.dispatchEvent(new Event('change', { bubbles: true }));
        } else {
          element.textContent = ${JSON.stringify(text)};
          element.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: ${JSON.stringify(text)} }));
        }
        return { ok: true, message: 'Typed into ${escapeForTemplate(ref)}' };
      })()
    `);
    await wait(180);
    const afterUrl = tab.view.webContents.getURL();
    const afterTitle = tab.view.webContents.getTitle();
    this.recordRecentEvent(tab, 'action', `${action} ${result.ok ? 'ok' : 'failed'}: ${result.message}`);
    const observation = await this.observeTab(tab);
    const verified = {
      ...result,
      action,
      ref,
      error: result.ok ? undefined : result.message,
      revalidated: true,
      beforeUrl,
      beforeTitle,
      afterUrl,
      afterTitle,
      changed: beforeUrl !== afterUrl || beforeTitle !== afterTitle,
      observedCandidateCount: observation.candidates.length,
      observedActions: observation.elements.filter((element) => element.interactable).length,
      observedLinks: observation.links.length,
      observedForms: observation.forms.length
    };
    this.actionHistory.unshift(verified);
    this.actionHistory = this.actionHistory.slice(0, 20);
    this.recordMemory('action', tab.id, afterUrl, tab.view.webContents.getTitle() || afterUrl, `${result.message}${verified.changed ? ' · changed' : ' · no page change'}`);
    return verified;
  }

  async scrollPage(direction: 'up' | 'down'): Promise<BrowserActionResult> {
    const delta = direction === 'up' ? '-Math.floor(window.innerHeight * 0.8)' : 'Math.floor(window.innerHeight * 0.8)';
    const beforeUrl = this.requireActiveTab().view.webContents.getURL();
    const beforeTitle = this.requireActiveTab().view.webContents.getTitle();
    const result = await this.executeActionScript(`
      (() => {
        window.scrollBy({ top: ${delta}, behavior: 'smooth' });
        return { ok: true, message: 'Scrolled ${direction}' };
      })()
    `);
    return { ...result, action: 'scroll', beforeUrl, beforeTitle, afterUrl: this.requireActiveTab().view.webContents.getURL(), afterTitle: this.requireActiveTab().view.webContents.getTitle() };
  }

  private async executePageReadScript(script: string): Promise<PageReadResult> {
    const result: unknown = await this.requireActiveTab().view.webContents.executeJavaScript(script, false);
    if (!isPageReadResult(result)) {
      throw new Error('Page script returned invalid result');
    }
    return result;
  }

  private async executeActionScript(script: string): Promise<BrowserActionResult> {
    return this.executeActionScriptInTab(this.requireActiveTab(), script);
  }

  private async executeActionScriptInTab(tab: RuntimeTab, script: string): Promise<BrowserActionResult> {
    const result: unknown = await tab.view.webContents.executeJavaScript(script, true);
    if (!isBrowserActionResult(result)) {
      throw new Error('Action script returned invalid result');
    }
    this.activateTab(tab.id);
    this.emitState();
    return result;
  }

  private async observeTab(tab: RuntimeTab): Promise<Omit<WorkspaceScanResult['pages'][number], 'tabId'>> {
    const result: unknown = await tab.view.webContents.executeJavaScript(WORKSPACE_SCAN_SCRIPT, false);
    if (!isWorkspacePageSummary(result)) throw new Error('Workspace scan returned invalid result');
    const accessibilityNodes = await this.captureAccessibilityNodes(tab);
    return { ...result, pageInfo: { ...result.pageInfo, browserState: { ...(result.pageInfo?.browserState ?? { loading: tab.view.webContents.isLoading(), crashed: tab.view.webContents.isCrashed(), pendingNetworkRequests: 0, permissions: {}, cookieNames: [], localStorageSamples: [], sessionStorageSamples: [] }), loading: tab.view.webContents.isLoading(), crashed: tab.view.webContents.isCrashed(), pendingNetworkRequests: tab.networkRequests.filter((request) => request.endedAt === null).length } }, consoleMessages: tab.consoleMessages, networkIssues: tab.networkIssues, networkRequests: tab.networkRequests, dialogs: tab.dialogs, popups: tab.popups, downloads: tab.downloads, recentEvents: tab.recentEvents, accessibilityNodeCount: accessibilityNodes.length, accessibilityNodes, elements: result.elements.map((element) => ({ ...element, ref: `${tab.id}:${element.ref}` })) };
  }

  private async captureAccessibilityNodes(tab: RuntimeTab): Promise<AccessibilityNodeSummary[]> {
    let attachedHere = false;
    try {
      if (!tab.view.webContents.debugger.isAttached()) {
        tab.view.webContents.debugger.attach('1.3');
        attachedHere = true;
      }
      const result = await tab.view.webContents.debugger.sendCommand('Accessibility.getFullAXTree');
    const nodes = (result as { nodes?: Array<{ role?: { value?: unknown }; name?: { value?: unknown }; value?: { value?: unknown }; description?: { value?: unknown }; properties?: Array<{ name?: string; value?: { value?: unknown } }> }> }).nodes;
      if (!Array.isArray(nodes)) return [];
      return nodes.map((node) => {
        const propertyValue = (name: string): unknown => node.properties?.find((property) => property.name === name)?.value?.value;
        return {
          role: String(node.role?.value ?? ''),
          name: String(node.name?.value ?? ''),
          value: String(node.value?.value ?? ''),
          description: String(node.description?.value ?? ''),
          disabled: Boolean(propertyValue('disabled')),
          checked: propertyValue('checked') === 'true' || propertyValue('checked') === true,
          selected: Boolean(propertyValue('selected')),
          expanded: Boolean(propertyValue('expanded')),
          pressed: Boolean(propertyValue('pressed')),
          focused: Boolean(propertyValue('focused')),
          required: Boolean(propertyValue('required')),
          invalid: Boolean(propertyValue('invalid')),
          level: typeof propertyValue('level') === 'number' ? propertyValue('level') as number : null,
        };
      }).filter((node) => node.role || node.name).slice(0, 80);
    } catch {
      return [];
    } finally {
      if (attachedHere && tab.view.webContents.debugger.isAttached()) tab.view.webContents.debugger.detach();
    }
  }

  private attachTabEvents(tab: RuntimeTab): void {
    const emit = () => this.emitState();
    tab.view.webContents.on('did-start-loading', emit);
    tab.view.webContents.on('did-start-loading', () => {
      tab.consoleMessages = [];
      tab.networkIssues = [];
      tab.networkRequests = [];
      this.recordRecentEvent(tab, 'load-start', tab.view.webContents.getURL());
    });
    tab.view.webContents.on('did-stop-loading', () => {
      this.recordRecentEvent(tab, 'load-stop', tab.view.webContents.getURL());
      emit();
    });
    tab.view.webContents.on('did-navigate', (_event, url) => {
      this.recordRecentEvent(tab, 'navigate', url);
      emit();
    });
    tab.view.webContents.on('did-navigate-in-page', (_event, url) => {
      this.recordRecentEvent(tab, 'navigate-in-page', url);
      emit();
    });
    tab.view.webContents.on('page-title-updated', emit);
    tab.view.webContents.on('page-favicon-updated', (_event, favicons) => {
      tab.favicon = favicons[0] ?? '';
      this.emitState();
    });
    tab.view.webContents.on('console-message', (_event, level, message, line, sourceId) => {
      tab.consoleMessages.unshift({ level: String(level), text: message.slice(0, 500), line, sourceId: sourceId.slice(0, 500) });
      tab.consoleMessages = tab.consoleMessages.slice(0, 30);
      this.recordRecentEvent(tab, 'console', `${level}: ${message}`.slice(0, 500));
    });
    tab.view.webContents.on('did-fail-load', (_event, _errorCode, errorDescription, validatedURL, isMainFrame) => {
      if (isMainFrame) return;
      tab.networkIssues.unshift({ url: validatedURL.slice(0, 500), error: errorDescription.slice(0, 240) });
      tab.networkIssues = tab.networkIssues.slice(0, 30);
      this.recordRecentEvent(tab, 'network-fail', `${validatedURL} ${errorDescription}`.slice(0, 500));
    });
    tab.view.webContents.on('did-start-navigation', (_event, url, isInPlace, isMainFrame, frameProcessId, frameRoutingId) => {
      if (!isMainFrame) return;
      tab.networkRequests.unshift({ id: `nav-${frameProcessId}-${frameRoutingId}-${Date.now()}`, url: url.slice(0, 500), method: 'GET', type: 'document', status: null, fromCache: false, startedAt: new Date().toISOString(), endedAt: null });
      tab.networkRequests = tab.networkRequests.slice(0, 50);
    });
    tab.view.webContents.on('found-in-page', (_event, result) => {
      if (tab.id === this.activeTabId && result.requestId === this.activeFindRequestId) this.eventTarget.send('browser:find-result', result);
    });
    tab.view.webContents.on('will-prevent-unload', (event) => {
      tab.dialogs.unshift({ kind: 'beforeunload', message: 'Page requested confirmation before unload', createdAt: new Date().toISOString() });
      tab.dialogs = tab.dialogs.slice(0, 20);
      this.recordRecentEvent(tab, 'dialog', 'beforeunload');
      event.preventDefault();
    });
    tab.view.webContents.setWindowOpenHandler((details) => {
      tab.popups.unshift({ url: details.url.slice(0, 500), frameName: details.frameName.slice(0, 120), disposition: details.disposition, createdAt: new Date().toISOString() });
      tab.popups = tab.popups.slice(0, 20);
      this.recordRecentEvent(tab, 'popup', details.url);
      return { action: 'deny' };
    });
    tab.view.webContents.session.on('will-download', (_event, item, webContents) => {
      if (webContents !== tab.view.webContents) return;
      const entry = { url: item.getURL().slice(0, 500), filename: item.getFilename().slice(0, 240), state: 'started', createdAt: new Date().toISOString() };
      tab.downloads.unshift(entry);
      tab.downloads = tab.downloads.slice(0, 20);
      this.recordRecentEvent(tab, 'download', `${entry.filename} ${entry.url}`.slice(0, 500));
      item.once('done', (_doneEvent, state) => {
        entry.state = state;
        this.recordRecentEvent(tab, 'download-done', `${entry.filename} ${state}`);
      });
    });
    tab.view.webContents.session.webRequest.onCompleted((details) => {
      const request = tab.networkRequests.find((entry) => entry.url === details.url.slice(0, 500) && entry.endedAt === null);
      if (request) {
        request.status = details.statusCode;
        request.fromCache = details.fromCache;
        request.endedAt = new Date().toISOString();
      }
    });
  }

  private recordRecentEvent(tab: RuntimeTab, type: string, detail: string): void {
    tab.recentEvents.unshift({ type, detail: detail.slice(0, 500), url: tab.view.webContents.getURL().slice(0, 500), createdAt: new Date().toISOString() });
    tab.recentEvents = tab.recentEvents.slice(0, 50);
  }

  private layoutActiveView(): void {
    if (!this.activeTabId) return;
    const tab = this.requireTab(this.activeTabId);
    const bounds = this.window.getBounds();
    tab.view.setBounds({
      x: 0,
      y: UI_TOP_HEIGHT,
      width: Math.max(200, bounds.width),
      height: Math.max(200, bounds.height - UI_TOP_HEIGHT)
    });
  }

  private toTabState(tab: RuntimeTab): BrowserTabState {
    const webContents = tab.view.webContents;
    return {
      id: tab.id,
      url: webContents.getURL(),
      title: webContents.getTitle() || webContents.getURL() || 'New Tab',
      favicon: tab.favicon,
      pinned: tab.pinned,
      loading: webContents.isLoading(),
      canGoBack: webContents.canGoBack(),
      canGoForward: webContents.canGoForward(),
      active: tab.id === this.activeTabId
    };
  }

  private emitState(): void {
    this.eventTarget.send('browser:state-changed', this.getState());
    this.scheduleSessionSave();
  }

  private scheduleSessionSave(): void {
    if (this.saveSessionTimer) clearTimeout(this.saveSessionTimer);
    this.saveSessionTimer = setTimeout(() => {
      this.saveSessionTimer = null;
      void this.saveSession();
    }, 250);
  }

  private async saveSession(): Promise<void> {
    const tabs = this.orderedTabs();
    const activeIndex = Math.max(0, tabs.findIndex((tab) => tab.id === this.activeTabId));
    const filePath = join(app.getPath('userData'), SESSION_FILE);
    await writeFile(filePath, JSON.stringify({ tabs: tabs.map((tab) => ({ url: tab.view.webContents.getURL(), pinned: tab.pinned })).filter((tab) => tab.url), activeIndex }, null, 2), 'utf8');
  }

  private async loadSession(): Promise<PersistedSession | null> {
    try {
      const filePath = join(app.getPath('userData'), SESSION_FILE);
      const parsed: unknown = JSON.parse(await readFile(filePath, 'utf8'));
      if (isPersistedSession(parsed)) return parsed;
      if (isLegacyPersistedSession(parsed)) return { tabs: parsed.urls.map((url) => ({ url, pinned: false })), activeIndex: parsed.activeIndex };
      return null;
    } catch {
      return null;
    }
  }

  private recordMemory(type: 'observation' | 'action', tabId: string, url: string, title: string, summary: string): void {
    this.workspaceMemory.unshift({ id: randomUUID(), type, tabId, url, title, summary, createdAt: new Date().toISOString() });
    this.workspaceMemory = this.workspaceMemory.slice(0, 50);
    void this.saveMemory();
  }

  private async saveMemory(): Promise<void> {
    const filePath = join(app.getPath('userData'), 'ai-workspace-browser-memory.json');
    const payload: PersistedMemory = { entries: this.workspaceMemory };
    await writeFile(filePath, JSON.stringify(payload, null, 2), 'utf8');
  }

  private async loadMemory(): Promise<void> {
    try {
      const filePath = join(app.getPath('userData'), 'ai-workspace-browser-memory.json');
      const parsed: unknown = JSON.parse(await readFile(filePath, 'utf8'));
      if (isPersistedMemory(parsed)) this.workspaceMemory = parsed.entries.slice(0, 50);
    } catch {
      this.workspaceMemory = [];
    }
  }

  private requireActiveTab(): RuntimeTab {
    if (!this.activeTabId) throw new Error('No active tab');
    return this.requireTab(this.activeTabId);
  }

  private requireTab(tabId: string): RuntimeTab {
    const tab = this.tabs.get(tabId);
    if (!tab) throw new Error(`Unknown tab: ${tabId}`);
    return tab;
  }

  private resolveObservedRef(ref: string): { tab: RuntimeTab; localRef: string } {
    const separatorIndex = ref.indexOf(':@w');
    if (separatorIndex === -1) return { tab: this.requireActiveTab(), localRef: ref };
    const tabId = ref.slice(0, separatorIndex);
    const localRef = ref.slice(separatorIndex + 1);
    return { tab: this.requireTab(tabId), localRef };
  }

  private orderedTabs(): RuntimeTab[] {
    return Array.from(this.tabs.values()).sort((left, right) => Number(right.pinned) - Number(left.pinned));
  }
}

function normalizeUrl(input: string): string {
  const trimmed = input.trim();
  if (/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) return trimmed;
  if (/^https?:\/\//i.test(trimmed)) return trimmed;
  if (trimmed.includes('.') && !trimmed.includes(' ')) return `https://${trimmed}`;
  return `https://www.google.com/search?q=${encodeURIComponent(trimmed)}`;
}

function createFixtureDataUrl(): string {
  const html = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Fixture Page</title>
    <meta name="description" content="Fixture page for runtime self test" />
  </head>
  <body>
    <h1>Fixture Page</h1>
    <p>Search me and inspect me.</p>
    <form id="fixture-form">
      <label for="fixture-input">Fixture input</label>
      <input id="fixture-input" name="fixture-input" placeholder="Type here" />
      <button id="fixture-apply" type="button">Apply</button>
    </form>
    <div id="fixture-output">Idle</div>
    <table>
      <thead><tr><th>Key</th><th>Value</th></tr></thead>
      <tbody><tr><td>One</td><td>Two</td></tr></tbody>
    </table>
    <a href="#target">Fixture link</a>
    <div id="target">Target</div>
    <script>
      const input = document.getElementById('fixture-input');
      const button = document.getElementById('fixture-apply');
      const output = document.getElementById('fixture-output');
      button.addEventListener('click', () => {
        output.textContent = 'Clicked:' + input.value;
      });
    </script>
  </body>
</html>`;
  return `data:text/html;charset=utf-8,${encodeURIComponent(html)}`;
}

function wait(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function isPageReadResult(value: unknown): value is PageReadResult {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.url === 'string' && typeof candidate.title === 'string' && typeof candidate.text === 'string';
}

const SNAPSHOT_SCRIPT = `
  (() => {
    const candidates = Array.from(document.querySelectorAll('a,button,input,textarea,select,[role="button"],[role="link"],[contenteditable="true"]'));
    window.__aiWorkspaceRefs = {};
    const elements = candidates.slice(0, 80).map((element, index) => {
      const rect = element.getBoundingClientRect();
      const ref = '@e' + (index + 1);
      window.__aiWorkspaceRefs[ref] = element;
      const inputValue = element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement ? element.value : '';
      const label = element.getAttribute('aria-label') || element.getAttribute('title') || element.getAttribute('placeholder') || '';
      return {
        ref,
        tag: element.tagName.toLowerCase(),
        role: element.getAttribute('role') || element.tagName.toLowerCase(),
        label,
        text: (element.textContent || '').replace(/\\s+/g, ' ').trim().slice(0, 160),
        inputValue: inputValue.slice(0, 160),
        visible: rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.right >= 0 && rect.top <= window.innerHeight && rect.left <= window.innerWidth
      };
    });
    return { url: location.href, title: document.title, elements };
  })()
`;

const STRUCTURED_CONTEXT_SCRIPT = `
  (() => {
    const isVisible = (element) => {
      const rect = element.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.right >= 0 && rect.top <= window.innerHeight && rect.left <= window.innerWidth;
    };

    const links = Array.from(document.querySelectorAll('a[href]')).slice(0, 120).map((link) => ({
      text: (link.textContent || link.getAttribute('aria-label') || '').replace(/\\s+/g, ' ').trim().slice(0, 180),
      href: link.href,
      visible: isVisible(link)
    })).filter((link) => link.href);

    const forms = Array.from(document.querySelectorAll('form')).slice(0, 20).map((form, index) => ({
      index,
      action: form.action || location.href,
      method: (form.method || 'get').toLowerCase(),
      fields: Array.from(form.querySelectorAll('input,textarea,select')).slice(0, 80).map((field) => {
        const id = field.id;
        const explicitLabel = id ? document.querySelector('label[for="' + CSS.escape(id) + '"]') : null;
        const wrappingLabel = field.closest('label');
        return {
          name: field.getAttribute('name') || '',
          type: field.getAttribute('type') || field.tagName.toLowerCase(),
          label: ((explicitLabel?.textContent || wrappingLabel?.textContent || field.getAttribute('aria-label') || '')).replace(/\\s+/g, ' ').trim().slice(0, 160),
          placeholder: field.getAttribute('placeholder') || '',
          value: field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement ? field.value.slice(0, 160) : ''
        };
      })
    }));

    const tables = Array.from(document.querySelectorAll('table')).slice(0, 20).map((table, index) => {
      const rows = Array.from(table.querySelectorAll('tr')).slice(0, 30).map((row) => Array.from(row.children).slice(0, 12).map((cell) => (cell.textContent || '').replace(/\\s+/g, ' ').trim().slice(0, 160)));
      const firstRow = rows[0] || [];
      const explicitHeaders = Array.from(table.querySelectorAll('th')).slice(0, 12).map((cell) => (cell.textContent || '').replace(/\\s+/g, ' ').trim().slice(0, 160));
      return {
        index,
        headers: explicitHeaders.length > 0 ? explicitHeaders : firstRow,
        rows: explicitHeaders.length > 0 ? rows : rows.slice(1)
      };
    });

    return { url: location.href, title: document.title, links, forms, tables };
  })()
`;

function isElementSnapshotResult(value: unknown): value is ElementSnapshotResult {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.url === 'string' && typeof candidate.title === 'string' && Array.isArray(candidate.elements);
}

function isBrowserActionResult(value: unknown): value is BrowserActionResult {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.ok === 'boolean' && typeof candidate.message === 'string';
}

function isStructuredPageContextResult(value: unknown): value is StructuredPageContextResult {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.url === 'string' && typeof candidate.title === 'string' && Array.isArray(candidate.links) && Array.isArray(candidate.forms) && Array.isArray(candidate.tables);
}

function isWorkspacePageSummary(value: unknown): value is Omit<WorkspaceScanResult['pages'][number], 'tabId'> {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.url === 'string' && typeof candidate.title === 'string' && typeof candidate.text === 'string' && Array.isArray(candidate.links) && Array.isArray(candidate.forms) && Array.isArray(candidate.elements) && typeof candidate.viewport === 'object';
}

const WORKSPACE_SCAN_SCRIPT = `
  (async () => {
    try {
    const isVisible = (element) => {
      const rect = element.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.right >= 0 && rect.top <= window.innerHeight && rect.left <= window.innerWidth;
    };
    window.__aiWorkspaceRefs = {};
    const fullText = (document.body?.innerText || '').replace(/\s+/g, ' ').trim();
    const text = fullText.slice(0, 5000);
    const roleFor = (element) => element.getAttribute('role') || (element instanceof HTMLAnchorElement ? 'link' : element instanceof HTMLButtonElement ? 'button' : element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement ? 'textbox' : element instanceof HTMLSelectElement ? 'combobox' : element.tagName.toLowerCase());
    const nameFor = (element) => (element.getAttribute('aria-label') || element.getAttribute('title') || element.getAttribute('placeholder') || element.getAttribute('alt') || element.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 140);
    const selectorFor = (element) => element.id ? '#' + CSS.escape(element.id) : element.getAttribute('data-testid') ? '[data-testid="' + CSS.escape(element.getAttribute('data-testid')) + '"]' : element.getAttribute('name') ? element.tagName.toLowerCase() + '[name="' + CSS.escape(element.getAttribute('name')) + '"]' : element.tagName.toLowerCase();
    const xpathFor = (element) => {
      const parts = [];
      let current = element;
      while (current && current.nodeType === Node.ELEMENT_NODE && current !== document.documentElement) {
        const tag = current.tagName.toLowerCase();
        const siblings = Array.from(current.parentElement?.children || []).filter((sibling) => sibling.tagName === current.tagName);
        const index = siblings.indexOf(current) + 1;
        parts.unshift(tag + (siblings.length > 1 ? '[' + index + ']' : ''));
        current = current.parentElement;
      }
      return '/html/' + parts.join('/');
    };
    const hashString = (value) => {
      let hash = 2166136261;
      for (let index = 0; index < value.length; index += 1) {
        hash ^= value.charCodeAt(index);
        hash = Math.imul(hash, 16777619);
      }
      return (hash >>> 0).toString(36);
    };
    const valueFor = (element) => element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element instanceof HTMLSelectElement ? String(element.value).slice(0, 160) : '';
    const booleanState = (element, name) => element.getAttribute(name) === 'true' || element.hasAttribute(name);
    const interactiveSelector = 'a[href],button,input,textarea,select,summary,[role="button"],[role="link"],[contenteditable="true"],[onclick]';
    const actionsFor = (element) => {
      if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element.isContentEditable) return ['type'];
      if (element instanceof HTMLSelectElement) return ['select'];
      if (element instanceof HTMLFormElement) return ['submit'];
      return ['click'];
    };
    const walkShadowRoots = (root, path, visitor) => {
      Array.from(root.querySelectorAll('*')).forEach((element) => {
        if (element.shadowRoot) {
          const nextPath = path.concat(selectorFor(element));
          visitor(element.shadowRoot, nextPath);
          walkShadowRoots(element.shadowRoot, nextPath, visitor);
        }
      });
    };
    const candidatesWithPaths = Array.from(document.querySelectorAll(interactiveSelector)).map((element) => ({ element, shadowPath: [] }));
    walkShadowRoots(document, [], (shadowRoot, shadowPath) => {
      Array.from(shadowRoot.querySelectorAll(interactiveSelector)).forEach((element) => candidatesWithPaths.push({ element, shadowPath }));
    });
    const candidates = candidatesWithPaths.slice(0, 120);
    const shadowRootCount = (() => {
      let count = 0;
      walkShadowRoots(document, [], () => { count += 1; });
      return count;
    })();
    const elements = candidates.map(({ element, shadowPath }, index) => {
      const rect = element.getBoundingClientRect();
      const style = window.getComputedStyle(element);
      const ref = '@w' + (index + 1);
      window.__aiWorkspaceRefs[ref] = element;
      const visible = isVisible(element) && style.visibility !== 'hidden' && style.display !== 'none';
      const actions = actionsFor(element);
      const stableSeed = [location.origin, xpathFor(element), selectorFor(element), roleFor(element), nameFor(element), element.getAttribute('name') || '', element.getAttribute('data-testid') || element.getAttribute('data-test') || ''].join('|');
      const identityHash = hashString(stableSeed);
      const centerX = Math.round(rect.left + rect.width / 2);
      const centerY = Math.round(rect.top + rect.height / 2);
      const topElement = centerX >= 0 && centerY >= 0 && centerX <= window.innerWidth && centerY <= window.innerHeight ? document.elementFromPoint(centerX, centerY) : null;
      const viewportArea = Math.max(1, window.innerWidth * window.innerHeight);
      const visibleWidth = Math.max(0, Math.min(rect.right, window.innerWidth) - Math.max(rect.left, 0));
      const visibleHeight = Math.max(0, Math.min(rect.bottom, window.innerHeight) - Math.max(rect.top, 0));
      return {
        ref,
        stableId: 'el_' + identityHash,
        identityHash,
        tag: element.tagName.toLowerCase(),
        role: roleFor(element),
        name: nameFor(element),
        text: (element.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 140),
        visible,
        interactable: visible && !element.disabled,
        disabled: Boolean(element.disabled) || booleanState(element, 'aria-disabled'),
        readonly: Boolean(element.readOnly) || booleanState(element, 'aria-readonly'),
        required: Boolean(element.required) || booleanState(element, 'aria-required'),
        checked: Boolean(element.checked) || element.getAttribute('aria-checked') === 'true',
        selected: Boolean(element.selected) || element.getAttribute('aria-selected') === 'true',
        expanded: element.getAttribute('aria-expanded') === 'true',
        value: valueFor(element),
        shadowPath,
        framePath: [location.href],
        locator: { id: element.id || '', testId: element.getAttribute('data-testid') || element.getAttribute('data-test') || '', name: element.getAttribute('name') || '', role: roleFor(element), text: (element.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 80) },
        actions,
        bounds: { x: Math.round(rect.x), y: Math.round(rect.y), width: Math.round(rect.width), height: Math.round(rect.height) },
        geometry: { centerX, centerY, viewportRatio: Math.round((visibleWidth * visibleHeight / viewportArea) * 10000) / 10000, cursor: style.cursor, overflow: style.overflow, scrollable: element.scrollHeight > element.clientHeight || element.scrollWidth > element.clientWidth, obscured: Boolean(topElement && topElement !== element && !element.contains(topElement)) },
        selectorHint: selectorFor(element)
      };
    });
    const xpathMap = Object.fromEntries(elements.map((element) => [element.ref, xpathFor(window.__aiWorkspaceRefs[element.ref])]));
    const urlMap = Object.fromEntries(elements.filter((element) => element.tag === 'a' && window.__aiWorkspaceRefs[element.ref] instanceof HTMLAnchorElement).map((element) => [element.ref, window.__aiWorkspaceRefs[element.ref].href]));
    const domSnapshot = {
      tree: elements.filter((element) => element.visible).slice(0, 80).map((element) => '[' + element.ref + ' ' + element.stableId + '] ' + element.role + ' "' + (element.name || element.text || element.selectorHint) + '" actions=' + element.actions.join('|')).join('\\n'),
      xpathMap,
      urlMap
    };
    const candidatesForAgent = elements.filter((element) => element.interactable).map((element, index) => ({
      selector: element.selectorHint,
      method: element.actions[0],
      description: (element.actions[0] || 'click') + ' ' + element.role + ' "' + (element.name || element.text || element.selectorHint) + '"',
      arguments: element.actions[0] === 'type' || element.actions[0] === 'select' ? ['%value%'] : [],
      rank: index + 1,
      ref: element.ref
    })).slice(0, 80);
    const links = Array.from(document.querySelectorAll('a[href]')).slice(0, 40).map((link) => ({
      text: (link.textContent || link.getAttribute('aria-label') || '').replace(/\s+/g, ' ').trim().slice(0, 120),
      href: link.href,
      visible: isVisible(link)
    })).filter((link) => link.href);
    const forms = Array.from(document.querySelectorAll('form')).slice(0, 8).map((form, index) => ({
      index,
      action: form.action || location.href,
      method: (form.method || 'get').toLowerCase(),
      fields: Array.from(form.querySelectorAll('input,textarea,select')).slice(0, 24).map((field) => ({
        name: field.getAttribute('name') || '',
        type: field.getAttribute('type') || field.tagName.toLowerCase(),
        label: (field.getAttribute('aria-label') || field.getAttribute('placeholder') || '').replace(/\s+/g, ' ').trim().slice(0, 120),
        placeholder: field.getAttribute('placeholder') || '',
        value: field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement ? field.value.slice(0, 120) : ''
      }))
    }));
    const tables = Array.from(document.querySelectorAll('table')).slice(0, 20).map((table, index) => {
      const rows = Array.from(table.querySelectorAll('tr')).slice(0, 20).map((row) => Array.from(row.children).slice(0, 10).map((cell) => (cell.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 120)));
      const explicitHeaders = Array.from(table.querySelectorAll('th')).slice(0, 10).map((cell) => (cell.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 120));
      return { index, headers: explicitHeaders.length ? explicitHeaders : rows[0] || [], rows: explicitHeaders.length ? rows : rows.slice(1) };
    });
    const headingText = (element) => (element.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 180);
    const focusedElement = document.activeElement && document.activeElement !== document.body ? { tag: document.activeElement.tagName.toLowerCase(), role: roleFor(document.activeElement), name: nameFor(document.activeElement) } : null;
    const activeElement = document.activeElement && document.activeElement !== document.body ? { tag: document.activeElement.tagName.toLowerCase(), role: roleFor(document.activeElement), name: nameFor(document.activeElement), xpath: xpathFor(document.activeElement), selector: selectorFor(document.activeElement) } : null;
    const resources = performance.getEntriesByType('resource');
    const resourceCount = (type) => resources.filter((resource) => resource.initiatorType === type).length;
    const duration = (entry) => entry ? Math.max(0, Math.round(entry.duration || 0)) : 0;
    const resourceSamples = resources.slice(-80).map((resource) => ({
      type: resource.initiatorType || 'other',
      url: resource.name,
      status: resource.responseStatus || 0,
      transferSize: Math.round(resource.transferSize || 0),
      durationMs: duration(resource)
    }));
    const summarizeFrameDocument = (frameDocument) => ({
      text: (frameDocument.body?.innerText || '').replace(/\s+/g, ' ').trim().slice(0, 1200),
      links: Array.from(frameDocument.querySelectorAll('a[href]')).slice(0, 20).map((link) => ({ text: (link.textContent || link.getAttribute('aria-label') || '').replace(/\s+/g, ' ').trim().slice(0, 120), href: link.href, visible: isVisible(link) })).filter((link) => link.href),
      forms: Array.from(frameDocument.querySelectorAll('form')).slice(0, 5).map((form, index) => ({ index, action: form.action || frameDocument.location.href, method: (form.method || 'get').toLowerCase(), fields: Array.from(form.querySelectorAll('input,textarea,select')).slice(0, 12).map((field) => ({ name: field.getAttribute('name') || '', type: field.getAttribute('type') || field.tagName.toLowerCase(), label: (field.getAttribute('aria-label') || field.getAttribute('placeholder') || '').replace(/\s+/g, ' ').trim().slice(0, 120), placeholder: field.getAttribute('placeholder') || '', value: field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement ? field.value.slice(0, 120) : '' })) })),
      headings: Array.from(frameDocument.querySelectorAll('h1,h2,h3,h4,h5,h6')).slice(0, 20).map((heading) => ({ level: Number(heading.tagName.slice(1)), text: (heading.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 160) })).filter((heading) => heading.text)
    });
    const frames = Array.from(document.querySelectorAll('iframe,frame')).slice(0, 40).map((frame, index) => {
      const rect = frame.getBoundingClientRect();
      const rawUrl = frame instanceof HTMLIFrameElement ? frame.src : frame.getAttribute('src') || '';
      let sameOrigin = false;
      let frameUrl = rawUrl;
      let detail = { text: '', links: [], forms: [], headings: [] };
      try {
        sameOrigin = Boolean(frame.contentWindow && frame.contentWindow.location.origin === location.origin);
        if (sameOrigin && frame.contentWindow) frameUrl = frame.contentWindow.location.href;
        if (sameOrigin && frame.contentDocument) detail = summarizeFrameDocument(frame.contentDocument);
      } catch {
        sameOrigin = false;
      }
      return {
        index,
        name: frame.getAttribute('name') || frame.getAttribute('id') || '',
        title: frame.getAttribute('title') || '',
        url: frameUrl,
        sameOrigin,
        visible: isVisible(frame),
        bounds: { x: Math.round(rect.x), y: Math.round(rect.y), width: Math.round(rect.width), height: Math.round(rect.height) },
        ...detail
      };
    });
    const navigation = performance.getEntriesByType('navigation')[0];
    const paints = performance.getEntriesByType('paint');
    const paintStart = (name) => paints.find((paint) => paint.name === name)?.startTime || 0;
    const storageSamples = (storageName) => {
      try {
        const storage = storageName === 'localStorage' ? window.localStorage : window.sessionStorage;
        return Object.keys(storage).slice(0, 20).map((key) => ({ key, valueLength: String(storage.getItem(key) || '').length }));
      } catch {
        return [];
      }
    };
    const cookieNames = (() => {
      try { return document.cookie.split(';').map((cookie) => cookie.split('=')[0].trim()).filter(Boolean).slice(0, 40); } catch { return []; }
    })();
    const permissionNames = ['geolocation', 'notifications', 'camera', 'microphone', 'clipboard-read', 'clipboard-write'];
    const permissions = {};
    if (navigator.permissions?.query) {
      await Promise.all(permissionNames.map(async (name) => {
        try {
          permissions[name] = (await navigator.permissions.query({ name })).state;
        } catch {
          permissions[name] = 'unsupported';
        }
      }));
    }
    const pageInfo = {
      language: document.documentElement.lang || '',
      readyState: document.readyState,
      origin: location.origin,
      protocol: location.protocol,
      secureContext: window.isSecureContext,
      description: document.querySelector('meta[name="description"]')?.getAttribute('content') || '',
      canonicalUrl: document.querySelector('link[rel="canonical"]')?.href || '',
      documentNodeCount: document.getElementsByTagName('*').length,
      shadowRootCount,
      textLength: fullText.length,
      scrollable: document.documentElement.scrollHeight > window.innerHeight || document.documentElement.scrollWidth > window.innerWidth,
      documentSize: { width: Math.max(document.documentElement.scrollWidth, document.body?.scrollWidth || 0), height: Math.max(document.documentElement.scrollHeight, document.body?.scrollHeight || 0) },
      focusedElement,
      activeElement,
      selectionText: String(window.getSelection()?.toString() || '').replace(/\s+/g, ' ').trim().slice(0, 500),
      headings: Array.from(document.querySelectorAll('h1,h2,h3,h4,h5,h6')).slice(0, 40).map((heading) => ({ level: Number(heading.tagName.slice(1)), text: headingText(heading) })).filter((heading) => heading.text),
      landmarks: Array.from(document.querySelectorAll('main,nav,header,footer,aside,section,[role="main"],[role="navigation"],[role="banner"],[role="contentinfo"],[role="complementary"],[role="region"]')).slice(0, 40).map((landmark) => ({ role: landmark.getAttribute('role') || landmark.tagName.toLowerCase(), label: (landmark.getAttribute('aria-label') || landmark.getAttribute('title') || headingText(landmark)).slice(0, 140) })),
      media: { images: document.images.length, videos: document.querySelectorAll('video').length, audios: document.querySelectorAll('audio').length, canvases: document.querySelectorAll('canvas').length, iframes: document.querySelectorAll('iframe').length },
      resources: { scripts: resourceCount('script'), stylesheets: resourceCount('link') + resourceCount('css'), fonts: resourceCount('css') + resourceCount('font'), fetches: resourceCount('fetch') + resourceCount('xmlhttprequest'), total: resources.length },
      storage: {
        cookies: (() => {
          try { return document.cookie ? document.cookie.split(';').filter(Boolean).length : 0; } catch { return 0; }
        })(),
        localStorageKeys: (() => {
          try { return Object.keys(localStorage).length; } catch { return 0; }
        })(),
        sessionStorageKeys: (() => {
          try { return Object.keys(sessionStorage).length; } catch { return 0; }
        })(),
        localStorageSampleKeys: (() => {
          try { return Object.keys(localStorage).slice(0, 40); } catch { return []; }
        })(),
        sessionStorageSampleKeys: (() => {
          try { return Object.keys(sessionStorage).slice(0, 40); } catch { return []; }
        })()
      },
      performance: { loadMs: Math.round(navigation?.loadEventEnd || 0), domContentLoadedMs: Math.round(navigation?.domContentLoadedEventEnd || 0), firstPaintMs: Math.round(paintStart('first-paint')), firstContentfulPaintMs: Math.round(paintStart('first-contentful-paint')), resourceDurationMs: resources.reduce((total, resource) => total + duration(resource), 0) },
      lifecycle: { visibilityState: document.visibilityState, hidden: document.hidden, prerendering: Boolean(document.prerendering) },
      frames,
      resourceSamples,
      browserState: { loading: document.readyState !== 'complete', crashed: false, pendingNetworkRequests: resources.filter((resource) => resource.responseEnd === 0).length, permissions, cookieNames, localStorageSamples: storageSamples('localStorage'), sessionStorageSamples: storageSamples('sessionStorage') }
    };
    return { url: location.href, title: document.title || location.href, text, links, forms, tables, pageInfo, viewport: { width: window.innerWidth, height: window.innerHeight, scrollX: window.scrollX, scrollY: window.scrollY }, domSnapshot, candidates: candidatesForAgent, elements };
    } catch (error) {
      return { url: location.href, title: document.title || location.href, text: '', links: [], forms: [], tables: [], pageInfo: null, viewport: { width: window.innerWidth, height: window.innerHeight, scrollX: window.scrollX, scrollY: window.scrollY }, domSnapshot: { tree: '', xpathMap: {}, urlMap: {} }, candidates: [], elements: [], error: error instanceof Error ? error.stack || error.message : String(error) };
    }
  })()
`;

function isPersistedSession(value: unknown): value is PersistedSession {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return Array.isArray(candidate.tabs) && candidate.tabs.every(isPersistedTab) && typeof candidate.activeIndex === 'number';
}

function isPersistedTab(value: unknown): value is { url: string; pinned: boolean } {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.url === 'string' && typeof candidate.pinned === 'boolean';
}

function isLegacyPersistedSession(value: unknown): value is { urls: string[]; activeIndex: number } {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return Array.isArray(candidate.urls) && candidate.urls.every((url) => typeof url === 'string') && typeof candidate.activeIndex === 'number';
}

function isPersistedMemory(value: unknown): value is PersistedMemory {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return Array.isArray(candidate.entries) && candidate.entries.every((entry) => typeof entry === 'object' && entry !== null);
}

function escapeForTemplate(value: string): string {
  return value.replace(/`/g, '\\`').replace(/\$/g, '\\$');
}

function safeFileName(value: string): string {
  const cleaned = value.replace(/[<>:"/\\|?*\u0000-\u001F]/g, '-').replace(/\s+/g, ' ').trim();
  return (cleaned || 'untitled-page').slice(0, 80);
}
