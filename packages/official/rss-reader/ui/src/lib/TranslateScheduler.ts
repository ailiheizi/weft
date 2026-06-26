/**
 * TranslateScheduler — 沉浸式翻译调度器
 * Phase 1: 立即翻译视口内段落
 * Phase 2: IntersectionObserver 监听剩余段落进入视口时翻译
 * 并发控制: 信号量 MAX_CONCURRENCY=3
 * 缓存: WeakSet<Element> 防止重复翻译
 */

export interface TranslateSchedulerOptions {
  iframe: HTMLIFrameElement;
  translateFn: (text: string) => Promise<string>;
  onProgress: (done: number, total: number) => void;
  signal: AbortSignal;
}

const STYLE_ID = 'weft-translate-style';
const TRANSLATE_STYLE = `
.weft-trans {
  color: #6b7280;
  font-style: italic;
  font-size: 0.9em;
  margin-top: 4px;
  border-left: 3px solid #3b82f6;
  padding-left: 10px;
  background: #f8fafc;
  border-radius: 0 4px 4px 0;
  padding: 6px 10px;
}
`;

const SELECTORS = 'p, h1, h2, h3, h4, h5, h6, li, blockquote, td, th';
const MIN_TEXT_LENGTH = 10;
const VIEWPORT_MARGIN = 200;
const MAX_CONCURRENCY = 3;

export class TranslateScheduler {
  private opts: TranslateSchedulerOptions;
  private translated = new WeakSet<Element>();
  private semaphore = 0;
  private done = 0;
  private total = 0;
  private observer: IntersectionObserver | null = null;
  private pendingElements: Set<Element> = new Set();

  constructor(opts: TranslateSchedulerOptions) {
    this.opts = opts;
  }

  async start(): Promise<void> {
    const doc = this.opts.iframe.contentDocument;
    if (!doc) return;

    // Inject translate style
    this.injectStyle(doc);

    // Collect all eligible paragraphs
    const allElements = Array.from(doc.querySelectorAll(SELECTORS)).filter(
      el => (el.textContent?.trim().length || 0) > MIN_TEXT_LENGTH
    );

    this.total = allElements.length;
    this.done = 0;
    this.opts.onProgress(0, this.total);

    if (this.opts.signal.aborted || this.total === 0) return;

    // Separate viewport vs off-screen elements
    const viewportHeight = this.opts.iframe.clientHeight;
    const inViewport: Element[] = [];
    const offScreen: Element[] = [];

    for (const el of allElements) {
      const rect = el.getBoundingClientRect();
      if (
        rect.bottom >= -VIEWPORT_MARGIN &&
        rect.top <= viewportHeight + VIEWPORT_MARGIN
      ) {
        inViewport.push(el);
      } else {
        offScreen.push(el);
      }
    }

    // Phase 1: Translate viewport elements immediately
    await this.translateBatch(inViewport);

    if (this.opts.signal.aborted) return;

    // Phase 2: IntersectionObserver for off-screen elements
    if (offScreen.length > 0) {
      this.setupObserver(doc, offScreen);
    }
  }

  stop(): void {
    // Disconnect observer
    if (this.observer) {
      this.observer.disconnect();
      this.observer = null;
    }
    this.pendingElements.clear();

    // Remove all translation nodes
    const doc = this.opts.iframe.contentDocument;
    if (doc) {
      doc.querySelectorAll('.weft-trans').forEach(el => el.remove());
      // Remove injected style
      const style = doc.getElementById(STYLE_ID);
      if (style) style.remove();
    }
  }

  private injectStyle(doc: Document): void {
    if (doc.getElementById(STYLE_ID)) return;
    const style = doc.createElement('style');
    style.id = STYLE_ID;
    style.textContent = TRANSLATE_STYLE;
    doc.head?.appendChild(style);
  }

  private setupObserver(_doc: Document, elements: Element[]): void {
    this.pendingElements = new Set(elements);

    this.observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting && this.pendingElements.has(entry.target)) {
            this.pendingElements.delete(entry.target);
            this.observer?.unobserve(entry.target);
            this.translateOne(entry.target);
          }
        }
      },
      {
        root: null,
        rootMargin: `${VIEWPORT_MARGIN}px`,
        threshold: 0,
      }
    );

    for (const el of elements) {
      if (this.opts.signal.aborted) break;
      this.observer.observe(el);
    }
  }

  private async translateBatch(elements: Element[]): Promise<void> {
    const promises = elements.map(el => this.translateOne(el));
    await Promise.all(promises);
  }

  private async translateOne(el: Element): Promise<void> {
    if (this.translated.has(el) || this.opts.signal.aborted) return;

    const text = el.textContent?.trim() || '';
    if (text.length < MIN_TEXT_LENGTH) return;

    // Wait for concurrency slot
    while (this.semaphore >= MAX_CONCURRENCY) {
      await new Promise(r => setTimeout(r, 50));
      if (this.opts.signal.aborted) return;
    }
    this.semaphore++;

    try {
      const result = await this.opts.translateFn(text);
      if (this.opts.signal.aborted) return;

      // Insert translated node
      const doc = this.opts.iframe.contentDocument;
      if (!doc) return;
      const transNode = doc.createElement('p');
      transNode.className = 'weft-trans';
      transNode.setAttribute('data-original-index', String(this.done));
      transNode.textContent = result;
      el.after(transNode);

      this.translated.add(el);
      this.done++;
      this.opts.onProgress(this.done, this.total);
    } catch {
      // Skip failed translations
      this.done++;
      this.opts.onProgress(this.done, this.total);
    } finally {
      this.semaphore--;
    }
  }
}
