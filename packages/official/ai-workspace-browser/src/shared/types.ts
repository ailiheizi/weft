export interface BrowserTabState {
  id: string;
  url: string;
  title: string;
  favicon: string;
  pinned: boolean;
  loading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  active: boolean;
}

export interface BrowserState {
  tabs: BrowserTabState[];
  activeTabId: string | null;
  canReopenClosedTab: boolean;
}

export interface PageReadResult {
  url: string;
  title: string;
  text: string;
}

export interface ScreenshotResult {
  path: string;
}

export interface SaveMarkdownResult {
  path: string;
  content: string;
}

export interface ElementSnapshotItem {
  ref: string;
  tag: string;
  role: string;
  label: string;
  text: string;
  inputValue: string;
  visible: boolean;
}

export interface ElementSnapshotResult {
  url: string;
  title: string;
  elements: ElementSnapshotItem[];
}

export interface PageLinkItem {
  text: string;
  href: string;
  visible: boolean;
}

export interface PageFormFieldItem {
  name: string;
  type: string;
  label: string;
  placeholder: string;
  value: string;
}

export interface PageFormItem {
  index: number;
  action: string;
  method: string;
  fields: PageFormFieldItem[];
}

export interface PageTableItem {
  index: number;
  headers: string[];
  rows: string[][];
}

export interface StructuredPageContextResult {
  url: string;
  title: string;
  links: PageLinkItem[];
  forms: PageFormItem[];
  tables: PageTableItem[];
}

export interface WorkspacePageSummary {
  tabId: string;
  url: string;
  title: string;
  text: string;
  links: PageLinkItem[];
  forms: PageFormItem[];
  tables: PageTableItem[];
  error?: string;
}

export interface ElementBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type ElementAction = 'click' | 'type' | 'select' | 'submit';

export interface WorkspaceElementObservation {
  ref: string;
  stableId: string;
  identityHash: string;
  tag: string;
  role: string;
  name: string;
  text: string;
  visible: boolean;
  interactable: boolean;
  disabled: boolean;
  readonly: boolean;
  required: boolean;
  checked: boolean;
  selected: boolean;
  expanded: boolean;
  value: string;
  shadowPath: string[];
  framePath: string[];
  locator: { id: string; testId: string; name: string; role: string; text: string };
  actions: ElementAction[];
  bounds: ElementBounds;
  geometry: { centerX: number; centerY: number; viewportRatio: number; cursor: string; overflow: string; scrollable: boolean; obscured: boolean };
  selectorHint: string;
}

export interface AccessibilityNodeSummary {
  role: string;
  name: string;
  value: string;
  description: string;
  disabled: boolean;
  checked: boolean;
  selected: boolean;
  expanded: boolean;
  pressed: boolean;
  focused: boolean;
  required: boolean;
  invalid: boolean;
  level: number | null;
}

export interface PageConsoleMessageSummary {
  level: string;
  text: string;
  line: number;
  sourceId: string;
}

export interface PageNetworkIssueSummary {
  url: string;
  error: string;
}

export interface PageDialogSummary {
  kind: string;
  message: string;
  createdAt: string;
}

export interface PagePopupSummary {
  url: string;
  frameName: string;
  disposition: string;
  createdAt: string;
}

export interface PageDownloadSummary {
  url: string;
  filename: string;
  state: string;
  createdAt: string;
}

export interface PageRecentEventSummary {
  type: string;
  detail: string;
  url: string;
  createdAt: string;
}

export interface PageNetworkRequestSummary {
  id: string;
  url: string;
  method: string;
  type: string;
  status: number | null;
  fromCache: boolean;
  startedAt: string;
  endedAt: string | null;
}

export interface PageHeadingSummary {
  level: number;
  text: string;
}

export interface PageLandmarkSummary {
  role: string;
  label: string;
}

export interface PageFrameSummary {
  index: number;
  name: string;
  title: string;
  url: string;
  sameOrigin: boolean;
  visible: boolean;
  bounds: ElementBounds;
  text: string;
  links: PageLinkItem[];
  forms: PageFormItem[];
  headings: PageHeadingSummary[];
}

export interface PageResourceSummary {
  type: string;
  url: string;
  status: number;
  transferSize: number;
  durationMs: number;
}

export interface PageStorageSampleSummary {
  key: string;
  valueLength: number;
}

export interface PageBrowserStateSummary {
  loading: boolean;
  crashed: boolean;
  pendingNetworkRequests: number;
  permissions: Record<string, string>;
  cookieNames: string[];
  localStorageSamples: PageStorageSampleSummary[];
  sessionStorageSamples: PageStorageSampleSummary[];
}

export interface PageDomSnapshotSummary {
  tree: string;
  xpathMap: Record<string, string>;
  urlMap: Record<string, string>;
}

export interface PageActionCandidateSummary {
  selector: string;
  method: ElementAction;
  description: string;
  arguments: string[];
  rank: number;
  ref: string;
}

export interface PageInformationSummary {
  language: string;
  readyState: string;
  origin: string;
  protocol: string;
  secureContext: boolean;
  description: string;
  canonicalUrl: string;
  documentNodeCount: number;
  shadowRootCount: number;
  textLength: number;
  scrollable: boolean;
  documentSize: { width: number; height: number };
  focusedElement: { tag: string; role: string; name: string } | null;
  activeElement: { tag: string; role: string; name: string; xpath: string; selector: string } | null;
  selectionText: string;
  headings: PageHeadingSummary[];
  landmarks: PageLandmarkSummary[];
  media: { images: number; videos: number; audios: number; canvases: number; iframes: number };
  resources: { scripts: number; stylesheets: number; fonts: number; fetches: number; total: number };
  storage: { cookies: number; localStorageKeys: number; sessionStorageKeys: number; localStorageSampleKeys: string[]; sessionStorageSampleKeys: string[] };
  performance: { loadMs: number; domContentLoadedMs: number; firstPaintMs: number; firstContentfulPaintMs: number; resourceDurationMs: number };
  lifecycle: { visibilityState: string; hidden: boolean; prerendering: boolean };
  frames: PageFrameSummary[];
  resourceSamples: PageResourceSummary[];
  browserState: PageBrowserStateSummary;
}

export interface WorkspacePageObservation extends WorkspacePageSummary {
  viewport: { width: number; height: number; scrollX: number; scrollY: number };
  pageInfo: PageInformationSummary;
  consoleMessages: PageConsoleMessageSummary[];
  networkIssues: PageNetworkIssueSummary[];
  networkRequests: PageNetworkRequestSummary[];
  dialogs: PageDialogSummary[];
  popups: PagePopupSummary[];
  downloads: PageDownloadSummary[];
  recentEvents: PageRecentEventSummary[];
  accessibilityNodeCount: number;
  accessibilityNodes: AccessibilityNodeSummary[];
  domSnapshot: PageDomSnapshotSummary;
  candidates: PageActionCandidateSummary[];
  elements: WorkspaceElementObservation[];
}

export interface WorkspaceMemoryEntry {
  id: string;
  type: 'observation' | 'action';
  tabId: string;
  url: string;
  title: string;
  summary: string;
  createdAt: string;
}

export interface WorkspaceMemoryResult {
  entries: WorkspaceMemoryEntry[];
}

export interface WorkspaceScanResult {
  pages: WorkspacePageObservation[];
}

export interface BrowserActionResult {
  ok: boolean;
  message: string;
  action?: ElementAction | 'scroll';
  ref?: string;
  error?: string;
  revalidated?: boolean;
  revalidationError?: string;
  changed?: boolean;
  beforeUrl?: string;
  beforeTitle?: string;
  afterUrl?: string;
  afterTitle?: string;
  observedCandidateCount?: number;
  observedActions?: number;
  observedLinks?: number;
  observedForms?: number;
}

export interface FindInPageResult {
  activeMatchOrdinal: number;
  matches: number;
  selectionArea?: ElementBounds;
}

export interface BrowserAPI {
  getState(): Promise<BrowserState>;
  createTab(url: string): Promise<BrowserState>;
  activateTab(tabId: string): Promise<BrowserState>;
  closeTab(tabId: string): Promise<BrowserState>;
  reopenClosedTab(): Promise<BrowserState>;
  duplicateTab(): Promise<BrowserState>;
  closeOtherTabs(): Promise<BrowserState>;
  togglePinTab(tabId?: string): Promise<BrowserState>;
  navigate(url: string): Promise<BrowserState>;
  goBack(): Promise<BrowserState>;
  goForward(): Promise<BrowserState>;
  reload(): Promise<BrowserState>;
  stop(): Promise<BrowserState>;
  findInPage(query: string, forward?: boolean): Promise<void>;
  stopFindInPage(): Promise<void>;
  readPage(): Promise<PageReadResult>;
  extractMarkdown(): Promise<PageReadResult>;
  captureScreenshot(): Promise<ScreenshotResult>;
  saveMarkdown(): Promise<SaveMarkdownResult>;
  getStructuredContext(): Promise<StructuredPageContextResult>;
  scanWorkspace(): Promise<WorkspaceScanResult>;
  getActionHistory(): Promise<BrowserActionResult[]>;
  getWorkspaceMemory(): Promise<WorkspaceMemoryResult>;
  searchWorkspaceMemory(query: string): Promise<WorkspaceMemoryResult>;
  snapshotElements(): Promise<ElementSnapshotResult>;
  clickRef(ref: string): Promise<BrowserActionResult>;
  typeRef(ref: string, text: string): Promise<BrowserActionResult>;
  actOnRef(ref: string, action: ElementAction, text?: string): Promise<BrowserActionResult>;
  scrollPage(direction: 'up' | 'down'): Promise<BrowserActionResult>;
  onFindResult(callback: (result: FindInPageResult) => void): () => void;
  onStateChanged(callback: (state: BrowserState) => void): () => void;
}

declare global {
  interface Window {
    browserAPI: BrowserAPI;
  }
}
