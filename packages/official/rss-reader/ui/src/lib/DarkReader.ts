/**
 * DarkReader — 精简版 dynamic 暗色引擎
 *
 * 不同于「静态 CSS 覆盖」（盲目把所有文字刷成浅灰、背景刷成深色），本引擎读取每个元素
 * 的 *实际计算颜色*（getComputedStyle 已把任何颜色归一化成 rgb()/rgba()），把每个颜色
 * 在感知亮度上做「翻转」，再以高优先级写回。这样能保留原页面的颜色语义（强调色、品牌色、
 * 代码高亮、表格分隔），同时让任何代理网页（arXiv / 少数派 / 博客）在暗色下都可读。
 *
 * 设计要点：
 *  1. 颜色转换走 HSL：背景色翻转明度并压暗、文字色翻转明度并提亮，色相/饱和度尽量保留。
 *  2. 只处理「与默认值不同」的属性，避免给每个元素都贴一堆 inline style。
 *  3. 图片/视频/canvas/svg 不反色，仅对「看起来是纯白插画/图标」的图片轻微降亮（可选）。
 *  4. 用一个 data 属性给每个改过的元素打标，stop() 时整体清除，零残留。
 *  5. 用 MutationObserver 处理懒加载/翻译节点等后插入的内容。
 */

export interface DarkReaderOptions {
  /** 背景基准明度（0-100），越小越深。默认 12。 */
  backgroundLightness?: number;
  /** 文字最低明度（0-100），保证对比度。默认 80。 */
  textLightness?: number;
  /** 是否给纯白/浅色图片降亮。默认 true。 */
  dimImages?: boolean;
}

interface RGBA { r: number; g: number; b: number; a: number; }
interface HSL { h: number; s: number; l: number; }

const MARK_ATTR = 'data-weft-dark';        // 标记被本引擎改过的元素
const STYLE_ID = 'weft-dark-engine';       // 注入到 <head> 的基础样式
const IMG_DIM_FILTER = 'brightness(0.85) contrast(1.05)';

// getComputedStyle 永远返回 rgb()/rgba()，所以只需解析这一种格式。
const RGB_RE = /^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*(?:,\s*([\d.]+)\s*)?\)$/;

export class DarkReader {
  private readonly doc: Document;
  private readonly bgL: number;
  private readonly textL: number;
  private readonly dimImages: boolean;
  private observer: MutationObserver | null = null;
  private active = false;

  constructor(doc: Document, options: DarkReaderOptions = {}) {
    this.doc = doc;
    this.bgL = options.backgroundLightness ?? 12;
    this.textL = options.textLightness ?? 80;
    this.dimImages = options.dimImages ?? true;
  }

  /** 启用暗色：注入基础样式 + 遍历现有 DOM + 监听后续变化。 */
  enable(): void {
    if (this.active) return;
    this.active = true;
    this.injectBaseStyle();
    this.processTree(this.doc.body);
    this.startObserver();
  }

  /** 关闭暗色：移除基础样式 + 还原所有 inline 覆盖 + 停止监听。零残留。 */
  disable(): void {
    if (!this.active) return;
    this.active = false;
    this.observer?.disconnect();
    this.observer = null;

    this.doc.getElementById(STYLE_ID)?.remove();

    // 清掉所有被打标元素的 inline 覆盖。我们只删自己写过的属性。
    const marked = this.doc.querySelectorAll(`[${MARK_ATTR}]`);
    marked.forEach((el) => {
      const node = el as HTMLElement;
      node.style.removeProperty('background-color');
      node.style.removeProperty('color');
      node.style.removeProperty('border-color');
      node.style.removeProperty('filter');
      node.removeAttribute(MARK_ATTR);
    });
  }

  // ── 颜色核心算法 ────────────────────────────────────────────────

  /** 解析 getComputedStyle 返回的 rgb()/rgba() 字符串。无法解析返回 null。 */
  private parseColor(value: string): RGBA | null {
    if (!value || value === 'transparent') return null;
    const m = RGB_RE.exec(value.trim());
    if (!m) return null;
    const a = m[4] !== undefined ? parseFloat(m[4]) : 1;
    return { r: +m[1], g: +m[2], b: +m[3], a };
  }

  private rgbToHsl({ r, g, b }: RGBA): HSL {
    const rn = r / 255, gn = g / 255, bn = b / 255;
    const max = Math.max(rn, gn, bn), min = Math.min(rn, gn, bn);
    const d = max - min;
    let h = 0;
    if (d !== 0) {
      if (max === rn) h = ((gn - bn) / d) % 6;
      else if (max === gn) h = (bn - rn) / d + 2;
      else h = (rn - gn) / d + 4;
      h *= 60;
      if (h < 0) h += 360;
    }
    const l = (max + min) / 2;
    const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
    return { h, s: s * 100, l: l * 100 };
  }

  private hslToCss({ h, s, l }: HSL, a: number): string {
    const sn = Math.max(0, Math.min(100, s));
    const ln = Math.max(0, Math.min(100, l));
    return a < 1
      ? `hsla(${h.toFixed(0)}, ${sn.toFixed(0)}%, ${ln.toFixed(0)}%, ${a})`
      : `hsl(${h.toFixed(0)}, ${sn.toFixed(0)}%, ${ln.toFixed(0)}%)`;
  }

  /**
   * 明度翻转核心：l' = 100 - l。这是 Dark Reader 的基本思路——
   * 浅色变深、深色变浅，色相保留，让强调色/品牌色仍可辨认。
   * 再按背景/前景角色做边界压缩。
   */
  private flip(l: number): number {
    return 100 - l;
  }

  /** 把一个背景色转成暗色：翻转后压到 bgL 附近，避免出现刺眼的中灰背景。 */
  private darkenBackground(c: RGBA): string {
    const hsl = this.rgbToHsl(c);
    let l = this.flip(hsl.l);
    // 原本越亮的背景（接近白），翻转后越接近 bgL；原本偏灰的背景适度抬升做层次。
    l = Math.min(l, this.bgL + hsl.l * 0.15);
    // 高饱和的彩色背景（如警示框）保留一点色彩但压暗。
    const s = hsl.s > 25 ? Math.min(hsl.s, 40) : hsl.s;
    return this.hslToCss({ h: hsl.h, s, l }, c.a);
  }

  /** 把一个文字色转成暗色下可读：翻转后抬到 textL 以上，保证对比度。 */
  private lightenForeground(c: RGBA): string {
    const hsl = this.rgbToHsl(c);
    let l = this.flip(hsl.l);
    // 原本越深的文字（接近黑），翻转后越接近 textL；浅色文字（已是强调色）少动。
    l = Math.max(l, this.textL - (100 - hsl.l) * 0.2);
    // 彩色文字（链接、强调）保留饱和度，仅保证够亮。
    const s = hsl.s;
    return this.hslToCss({ h: hsl.h, s, l }, c.a);
  }

  /** 边框/分隔线：统一压成低对比的暗灰，保留一点原色相。 */
  private darkenBorder(c: RGBA): string {
    const hsl = this.rgbToHsl(c);
    return this.hslToCss({ h: hsl.h, s: Math.min(hsl.s, 15), l: this.bgL + 12 }, c.a);
  }

  // ── DOM 处理 ────────────────────────────────────────────────────

  private injectBaseStyle(): void {
    if (this.doc.getElementById(STYLE_ID)) return;
    const bg = `hsl(0,0%,${this.bgL}%)`;
    const fg = `hsl(0,0%,${this.textL}%)`;
    const style = this.doc.createElement('style');
    style.id = STYLE_ID;
    // 基础兜底：html/body 给暗背景；color-scheme 让原生控件/滚动条也走暗色；
    // 图片降亮用属性选择器，悬停恢复原亮度（细节体验）。
    style.textContent = `
      :root { color-scheme: dark !important; }
      html, body {
        background-color: ${bg} !important;
        color: ${fg} !important;
      }
      ::selection { background: hsl(217,80%,40%); color: #fff; }
      ${this.dimImages ? `
      img, video, picture, canvas, [style*="background-image"] {
        filter: ${IMG_DIM_FILTER};
        transition: filter 0.15s;
      }
      img:hover, video:hover, picture:hover { filter: none; }
      ` : ''}
      /* 翻译节点：暗色下用半透明蓝边 + 浅灰字，保持与正文区分但可读 */
      .weft-trans {
        color: hsl(220,12%,72%) !important;
        background: hsla(217,30%,20%,0.4) !important;
        border-left-color: hsl(217,80%,60%) !important;
      }
    `;
    this.doc.head?.appendChild(style);
  }

  /** 遍历元素树，对每个元素按计算样式生成 inline 暗色覆盖。 */
  private processTree(root: Element | null): void {
    if (!root) return;
    const walker = this.doc.createTreeWalker(root, NodeFilter.SHOW_ELEMENT);
    let node: Node | null = root;
    while (node) {
      this.processElement(node as HTMLElement);
      node = walker.nextNode();
    }
  }

  private processElement(el: HTMLElement): void {
    const tag = el.tagName;
    // 跳过媒体/脚本/样式/翻译节点（翻译节点交给基础样式处理）。
    if (
      tag === 'IMG' || tag === 'VIDEO' || tag === 'CANVAS' || tag === 'SVG' ||
      tag === 'PICTURE' || tag === 'SCRIPT' || tag === 'STYLE' || tag === 'IFRAME' ||
      el.classList.contains('weft-trans')
    ) {
      return;
    }

    const cs = this.doc.defaultView?.getComputedStyle(el);
    if (!cs) return;

    let changed = false;

    // 背景：只在元素自己声明了非透明背景时才覆盖（否则让父级背景透出，减少 inline 噪声）。
    const bg = this.parseColor(cs.backgroundColor);
    if (bg && bg.a > 0) {
      el.style.setProperty('background-color', this.darkenBackground(bg), 'important');
      changed = true;
    }

    // 文字：几乎所有元素都需要（继承也算，但显式写回最稳，能压过页面 inline style）。
    const fg = this.parseColor(cs.color);
    if (fg && fg.a > 0) {
      el.style.setProperty('color', this.lightenForeground(fg), 'important');
      changed = true;
    }

    // 边框：仅在有可见边框时处理。
    const bc = this.parseColor(cs.borderTopColor);
    if (bc && bc.a > 0 && parseFloat(cs.borderTopWidth) > 0) {
      el.style.setProperty('border-color', this.darkenBorder(bc), 'important');
      changed = true;
    }

    if (changed) el.setAttribute(MARK_ATTR, '1');
  }

  private startObserver(): void {
    this.observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        m.addedNodes.forEach((n) => {
          if (n.nodeType === Node.ELEMENT_NODE) {
            this.processTree(n as Element);
          }
        });
      }
    });
    if (this.doc.body) {
      this.observer.observe(this.doc.body, { childList: true, subtree: true });
    }
  }

  /** 静态清理：移除注入样式 + 还原所有打标元素的 inline 覆盖。与实例状态无关。 */
  static purge(doc: Document): void {
    doc.getElementById(STYLE_ID)?.remove();
    doc.querySelectorAll(`[${MARK_ATTR}]`).forEach((el) => {
      const node = el as HTMLElement;
      node.style.removeProperty('background-color');
      node.style.removeProperty('color');
      node.style.removeProperty('border-color');
      node.style.removeProperty('filter');
      node.removeAttribute(MARK_ATTR);
    });
  }
}

// ── 便捷函数：按 document 缓存实例，供 React 组件直接调用 ──
const instances = new WeakMap<Document, DarkReader>();

export function applyDarkMode(doc: Document, options?: DarkReaderOptions): void {
  let inst = instances.get(doc);
  if (!inst) {
    inst = new DarkReader(doc, options);
    instances.set(doc, inst);
  }
  inst.enable();
}

export function removeDarkMode(doc: Document): void {
  const inst = instances.get(doc);
  if (inst) {
    inst.disable();
    instances.delete(doc);
  } else {
    // 兜底：无缓存实例（如热重载、iframe 重新 load 后）也清掉残留样式与标记
    DarkReader.purge(doc);
  }
}


