import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

const root = process.cwd();
const files = {
  packageJson: await readFile(join(root, 'package.json'), 'utf8'),
  pluginJson: await readFile(join(root, 'plugin.json'), 'utf8'),
  plugin: await readFile(join(root, 'src/plugin/plugin.ts'), 'utf8'),
  runtime: await readFile(join(root, 'src/main/browser-runtime.ts'), 'utf8'),
  types: await readFile(join(root, 'src/shared/types.ts'), 'utf8'),
  renderer: await readFile(join(root, 'src/renderer/App.tsx'), 'utf8'),
  preload: await readFile(join(root, 'src/preload/preload.ts'), 'utf8'),
  ipc: await readFile(join(root, 'src/main/ipc.ts'), 'utf8')
};

const checks = [
  ['Package is shaped as Weft browser plugin', files.packageJson.includes('@weft/plugin-ai-workspace-browser') && files.pluginJson.includes('weft.ai-workspace-browser') && files.plugin.includes('createAiWorkspaceBrowserPlugin')],
  ['Plugin declares screenshot-free browser context policy', files.pluginJson.includes('"defaultScreenshots": false') && files.plugin.includes('browserNativeContextOnly: true')],
  ['BrowserState exposes reopen affordance', files.types.includes('canReopenClosedTab: boolean')],
  ['Runtime tracks recently closed tabs', files.runtime.includes('recentlyClosedTabs') && files.runtime.includes('reopenClosedTab()')],
  ['Renderer exposes Ctrl/Cmd+Shift+T', /event\.shiftKey[\s\S]+reopenClosedTab\(\)/.test(files.renderer)],
  ['Browser exposes native find in page', files.types.includes('FindInPageResult') && files.runtime.includes('findInPage(trimmed') && files.preload.includes('onFindResult') && files.renderer.includes('find-bar')],
  ['Find in page tracks request IDs and findNext', files.runtime.includes('activeFindRequestId') && files.runtime.includes('result.requestId === this.activeFindRequestId') && files.runtime.includes('const findNext = trimmed === this.lastFindQuery')],
  ['Observed refs are tab-qualified', /ref:\s*`\$\{tab\.id\}:\$\{element\.ref\}`/.test(files.runtime)],
  ['Grounded actions route by observed tab ref', files.runtime.includes('resolveObservedRef(ref)') && files.runtime.includes('executeActionScriptInTab(tab')],
  ['Action verification re-observes target tab', files.runtime.includes('const observation = await this.observeTab(tab)')],
  ['Action results keep structured diagnostics', files.types.includes("action?: ElementAction | 'scroll'") && files.types.includes('beforeTitle?: string') && files.types.includes('observedCandidateCount?: number') && files.runtime.includes('observedCandidateCount: observation.candidates.length')],
  ['Actions revalidate refs before execution', files.types.includes('revalidated?: boolean') && files.types.includes('revalidationError?: string') && files.runtime.includes('Observed ref revalidated') && files.runtime.includes('Observed ref is no longer visible')],
  ['Workspace memory persists and searches', files.runtime.includes('saveMemory()') && files.runtime.includes('searchWorkspaceMemory(query')],
  ['Pinned tabs use consistent ordering for state/session', files.runtime.includes('orderedTabs()') && files.runtime.includes('const tabs = this.orderedTabs();')],
  ['Observe captures accessibility coverage', files.types.includes('accessibilityNodeCount: number') && files.runtime.includes('Accessibility.getFullAXTree')],
  ['Observe does not auto-capture screenshots', !files.types.includes('screenshotPath: string') && !files.runtime.includes('captureTabScreenshot(tab)')],
  ['Observe stores compact accessibility summaries', files.types.includes('AccessibilityNodeSummary') && files.runtime.includes('captureAccessibilityNodes(tab)')],
  ['Observe stores rich AX attributes', files.types.includes('pressed: boolean') && files.runtime.includes("propertyValue('disabled')") && files.runtime.includes("propertyValue('level')")],
  ['Observe stores browser-use-style element state and locators', files.types.includes('framePath: string[]') && files.runtime.includes('data-testid') && files.runtime.includes('valueFor(element)')],
  ['Observe stores stable element identity', files.types.includes('stableId: string') && files.types.includes('identityHash: string') && files.runtime.includes('hashString') && files.runtime.includes('stableSeed')],
  ['Observe stores geometry and clickability signals', files.types.includes('viewportRatio: number') && files.types.includes('obscured: boolean') && files.runtime.includes('elementFromPoint') && files.runtime.includes('scrollHeight > element.clientHeight')],
  ['Observe stores compact DOM snapshot and action candidates', files.types.includes('PageDomSnapshotSummary') && files.types.includes('PageActionCandidateSummary') && files.runtime.includes('domSnapshot') && files.runtime.includes('candidatesForAgent')],
  ['Observe traverses shadow DOM context', files.types.includes('shadowPath: string[]') && files.types.includes('shadowRootCount: number') && files.runtime.includes('walkShadowRoots')],
  ['Accessibility debugger detaches after own attach', files.runtime.includes('let attachedHere = false') && files.runtime.includes('debugger.detach()')],
  ['Observe stores rich page information', files.types.includes('PageInformationSummary') && files.runtime.includes('documentNodeCount') && files.runtime.includes('headings: Array.from')],
  ['Observe stores active element context', files.types.includes('activeElement: { tag: string; role: string; name: string; xpath: string; selector: string } | null') && files.runtime.includes('activeElement = document.activeElement') && files.runtime.includes('xpath: xpathFor(document.activeElement)')],
  ['Observe stores technical page state', files.types.includes('secureContext: boolean') && files.runtime.includes("performance.getEntriesByType('resource')") && files.renderer.includes('resources')],
  ['Observe stores storage key samples', files.types.includes('localStorageSampleKeys: string[]') && files.types.includes('sessionStorageSampleKeys: string[]') && files.runtime.includes('Object.keys(localStorage).slice')],
  ['Observe stores compact resource inventory', files.types.includes('PageResourceSummary') && files.runtime.includes('resourceSamples') && files.runtime.includes('responseStatus')],
  ['Observe stores runtime console and network issues', files.types.includes('PageConsoleMessageSummary') && files.runtime.includes("webContents.on('console-message'") && files.runtime.includes("webContents.on('did-fail-load'")],
  ['Observe stores network request summaries', files.types.includes('PageNetworkRequestSummary') && files.runtime.includes('did-start-navigation') && files.runtime.includes('webRequest.onCompleted')],
  ['Observe stores browser state shell', files.types.includes('PageBrowserStateSummary') && files.types.includes('pendingNetworkRequests: number') && files.runtime.includes('navigator.permissions?.query') && files.runtime.includes('cookieNames') && files.runtime.includes('localStorageSamples')],
  ['Observe stores transient browser state', files.types.includes('PageDialogSummary') && files.types.includes('PagePopupSummary') && files.types.includes('PageDownloadSummary') && files.runtime.includes('setWindowOpenHandler') && files.runtime.includes("session.on('will-download'")],
  ['Observe stores recent event stream', files.types.includes('PageRecentEventSummary') && files.runtime.includes('recordRecentEvent(tab') && files.runtime.includes('recentEvents: tab.recentEvents')],
  ['Observe stores performance and lifecycle timings', files.types.includes('firstContentfulPaintMs') && files.runtime.includes("performance.getEntriesByType('navigation')") && files.runtime.includes('document.visibilityState')],
  ['Observe stores frame context', files.types.includes('PageFrameSummary') && files.runtime.includes("document.querySelectorAll('iframe,frame')") && files.runtime.includes('sameOrigin')],
  ['Observe stores same-origin frame details', files.types.includes('text: string;\n  links: PageLinkItem[];\n  forms: PageFormItem[];\n  headings: PageHeadingSummary[];') && files.runtime.includes('summarizeFrameDocument') && files.runtime.includes('frame.contentDocument')],
  ['Observe stores workspace table data', files.types.includes('tables: PageTableItem[]') && files.runtime.includes("document.querySelectorAll('table')") && files.renderer.includes('tables')],
  ['Runtime disposes WebContentsView tabs on window close', files.runtime.includes('dispose(): void') && files.runtime.includes('webContents.close()') && files.ipc.includes('registerBrowserIpc')],
  ['IPC/preload expose memory and reopen APIs', files.ipc.includes('browser:reopen-closed-tab') && files.preload.includes('searchWorkspaceMemory')]
];

const failed = checks.filter(([, ok]) => !ok);
for (const [name, ok] of checks) {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
}

if (failed.length) {
  process.exitCode = 1;
}
