import { useState, useEffect, useRef, useCallback } from 'react';
import {
  Heart, Languages, Bot, Settings, Maximize, HelpCircle,
  ArrowLeft, FileText, ExternalLink, X, Loader2, Moon,
  ChevronLeft, ChevronRight, RotateCw,
} from 'lucide-react';
import { api } from '../api';
import type { Article } from '../api';
import { TranslateScheduler } from '../lib/TranslateScheduler';
import { applyDarkMode, removeDarkMode } from '../lib/DarkReader';

interface TranslateProgress {
  done: number;
  total: number;
}

interface Props {
  article: Article | null;
  onToggleFavorite: (a: Article) => void;
  onBackToList: () => void;
  onToggleAiPanel: () => void;
  onOpenSettings: () => void;
  onToggleZen: () => void;
  onShowShortcuts: () => void;
  isZen: boolean;
  darkMode: boolean;
  onToggleDarkMode: () => void;
  fontSize: number;
}

const FONT_STYLE_ID = 'weft-font-size';

export function ReaderPane({
  article, onToggleFavorite, onBackToList,
  onToggleAiPanel, onOpenSettings, onToggleZen, onShowShortcuts, isZen,
  darkMode, onToggleDarkMode, fontSize,
}: Props) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [frameMode, setFrameMode] = useState<'loading' | 'proxy' | 'direct' | 'error'>('loading');
  const [proxiedHtml, setProxiedHtml] = useState('');
  const [proxyError, setProxyError] = useState('');
  // 导航历史栈
  const [navHistory, setNavHistory] = useState<string[]>([]);
  const [navIndex, setNavIndex] = useState(-1);
  const [translateState, setTranslateState] = useState<'idle' | 'translating' | 'done'>('idle');
  const [translateProgress, setTranslateProgress] = useState<TranslateProgress | null>(null);
  const [translateNotice, setTranslateNotice] = useState('');
  const abortRef = useRef<AbortController | null>(null);
  const schedulerRef = useRef<TranslateScheduler | null>(null);
  const [selectionPopup, setSelectionPopup] = useState<{ x: number; y: number; text: string } | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // 用 ref 持有最新的暗色/字号偏好，供 iframe onLoad 回调读取（闭包不会拿到旧值）。
  const darkModeRef = useRef(darkMode);
  darkModeRef.current = darkMode;
  const fontSizeRef = useRef(fontSize);
  fontSizeRef.current = fontSize;

  // 把字号偏好以高优先级注入 iframe 正文，覆盖 readingStyle 的固定 16px。
  const applyFontSize = useCallback((doc: Document, size: number) => {
    let style = doc.getElementById(FONT_STYLE_ID) as HTMLStyleElement | null;
    if (!style) {
      style = doc.createElement('style');
      style.id = FONT_STYLE_ID;
      doc.head?.appendChild(style);
    }
    style.textContent = `body { font-size: ${size}px !important; }`;
  }, []);

  // 统一的「把当前阅读偏好应用到 iframe 文档」逻辑，从 ref 读取最新值。
  const applyReaderPrefs = useCallback(() => {
    const doc = iframeRef.current?.contentDocument;
    if (!doc || !doc.body) return;
    applyFontSize(doc, fontSizeRef.current);
    if (darkModeRef.current) {
      applyDarkMode(doc);
    } else {
      removeDarkMode(doc);
    }
  }, [applyFontSize]);

  // iframe load 完成回调：此时 srcDoc 已解析，contentDocument 可用。
  const handleIframeLoad = useCallback(() => {
    applyReaderPrefs();
  }, [applyReaderPrefs]);

  // 暗色开关切换：立即对当前文档生效（无需等下次 load）。
  useEffect(() => {
    const doc = iframeRef.current?.contentDocument;
    if (!doc || !doc.body) return;
    if (darkMode) applyDarkMode(doc);
    else removeDarkMode(doc);
  }, [darkMode, frameMode, proxiedHtml]);

  // 字号切换：立即生效。
  useEffect(() => {
    const doc = iframeRef.current?.contentDocument;
    if (!doc || !doc.body) return;
    applyFontSize(doc, fontSize);
  }, [fontSize, frameMode, proxiedHtml, applyFontSize]);

  useEffect(() => {
    setFrameMode('loading');
    setProxiedHtml('');
    setTranslateState('idle');
    setTranslateProgress(null);
    setTranslateNotice('');
    setProxyError('');
    // Abort any in-progress translation
    abortRef.current?.abort();
    schedulerRef.current = null;
    if (article?.link) {
      setNavHistory([article.link]);
      setNavIndex(0);
      loadProxy(article.link);
    }
  }, [article?.id]);

  // 监听 iframe 内的导航请求（postMessage）
  useEffect(() => {
    const handler = (e: MessageEvent) => {
      if (e.data?.type === 'weft-navigate' && e.data?.url) {
        navigateTo(e.data.url);
      }
    };
    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [navHistory, navIndex]);

  // 监听「跳转到原文某段」请求（来自 AI 面板的分段大纲点击）。
  // 用 TreeWalker 在 iframe 文档里找含目标文本的文本节点，scrollIntoView + 高亮闪烁 2s。
  // 找不到时派发 rss-scroll-result 事件回传失败，由面板提示。
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail || {};
      const target: string = (detail.text || '').trim();
      const found = scrollToText(target);
      window.dispatchEvent(new CustomEvent('rss-scroll-result', { detail: { index: detail.index, found } }));
    };
    window.addEventListener('rss-scroll-to-text', handler);
    return () => window.removeEventListener('rss-scroll-to-text', handler);
  }, []);

  // Setup selection listener on iframe（仅划词，不再拦截链接——链接由 iframe 内注入脚本处理，避免双重导航）
  useEffect(() => {
    const iframe = iframeRef.current;
    if (!iframe || frameMode === 'loading') return;

    const setupListener = () => {
      const doc = iframe.contentDocument;
      if (!doc) return;

      const handler = () => {
        const sel = doc.getSelection();
        const text = sel?.toString().trim() || '';
        if (text.length > 2) {
          const range = sel!.getRangeAt(0);
          const rect = range.getBoundingClientRect();
          // Position relative to iframe
          const iframeRect = iframe.getBoundingClientRect();
          setSelectionPopup({
            x: rect.left + rect.width / 2 - iframeRect.left,
            y: rect.top - iframeRect.top - 40,
            text,
          });
        } else {
          setSelectionPopup(null);
        }
      };

      doc.addEventListener('mouseup', handler);
      // Dismiss on click elsewhere
      doc.addEventListener('mousedown', () => setSelectionPopup(null));

      return () => {
        doc.removeEventListener('mouseup', handler);
      };
    };

    // Wait for iframe load
    const timer = setTimeout(setupListener, 500);
    iframe.addEventListener('load', setupListener);
    return () => {
      clearTimeout(timer);
      iframe.removeEventListener('load', setupListener);
    };
  }, [frameMode, proxiedHtml]);

  const loadProxy = async (url: string) => {
    setFrameMode('loading');
    setProxyError('');

    // PDF 检测：用 Mozilla PDF.js viewer 渲染（不走 proxy，不走 embed）
    if (url.match(/\.pdf(\?|#|$)/i) || url.includes('/pdf/')) {
      const viewerUrl = `https://mozilla.github.io/pdf.js/web/viewer.html?file=${encodeURIComponent(url)}`;
      setProxiedHtml(`
        <html><body style="margin:0;height:100vh;overflow:hidden;">
          <iframe src="${viewerUrl}" style="width:100%;height:100%;border:none;"></iframe>
        </body></html>
      `);
      setFrameMode('proxy');
      return;
    }

    try {
      const res = await api.proxyPage(url);
      const base = `<base href="${url}" />`;
      const readingStyle = `<style>
        body { max-width: 720px; margin: 0 auto; font-size: 16px; line-height: 1.8; padding: 24px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; color: #1a1a1a; background: #ffffff; }
        img { max-width: 100%; height: auto; }
        pre { overflow-x: auto; background: #f5f5f5; padding: 12px; border-radius: 6px; }
        code { font-size: 0.9em; }
        .weft-trans { color: #6b7280; font-style: italic; font-size: 0.9em; margin-top: 4px; border-left: 2px solid #3b82f6; padding-left: 8px; }
        .weft-selection-popup { position: absolute; background: #1e1e28; border: 1px solid #2a2a3a; border-radius: 8px; padding: 4px; display: flex; gap: 4px; z-index: 9999; box-shadow: 0 4px 12px rgba(0,0,0,0.4); }
        .weft-selection-popup button { padding: 4px 8px; border: none; border-radius: 4px; font-size: 11px; cursor: pointer; color: #e5e7eb; background: #2a2a3a; }
        .weft-selection-popup button:hover { background: #3b82f6; }
      </style>`;
      // 注入导航拦截脚本：覆盖 window.open / 拦截所有链接 / 阻止 form 提交离开页面
      const navInterceptScript = `<script>
        (function(){
          // 覆盖 window.open
          window.open = function(url) {
            if (url && (url.startsWith('http://') || url.startsWith('https://'))) {
              window.parent.postMessage({type:'weft-navigate', url: url}, '*');
            }
            return null;
          };
          // 拦截所有点击（包括 target=_blank）
          document.addEventListener('click', function(e) {
            var a = e.target.closest ? e.target.closest('a') : null;
            if (a && a.href && (a.href.startsWith('http://') || a.href.startsWith('https://'))) {
              e.preventDefault();
              e.stopPropagation();
              window.parent.postMessage({type:'weft-navigate', url: a.href}, '*');
            }
          }, true);
          // 拦截表单提交
          document.addEventListener('submit', function(e) {
            e.preventDefault();
          }, true);
        })();
      </script>`;
      const html = res.html.replace(/<head[^>]*>/i, (match: string) => `${match}${base}${readingStyle}${navInterceptScript}`);
      setProxiedHtml(html);
      setFrameMode('proxy');
    } catch (e: any) {
      console.error('proxy_page failed:', e);
      setProxyError(e?.message || String(e) || '代理加载失败');
      setFrameMode('error');
    }
  };

  const navigateTo = (url: string) => {
    const newHistory = navHistory.slice(0, navIndex + 1);
    newHistory.push(url);
    setNavHistory(newHistory);
    setNavIndex(newHistory.length - 1);
    loadProxy(url);
  };

  // 在 iframe 文档里找到含 `target` 的文本节点，滚动过去并高亮闪烁 2 秒。返回是否命中。
  const scrollToText = (target: string): boolean => {
    const doc = iframeRef.current?.contentDocument;
    if (!doc || !doc.body || !target) return false;
    const needle = target.replace(/\s+/g, ' ').trim().slice(0, 40);
    if (!needle) return false;

    const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_TEXT);
    let hit: HTMLElement | null = null;
    let node: Node | null = walker.nextNode();
    // 先全量匹配，未命中则用前 12 字做前缀宽松匹配（AI 给的 start_text 可能与原文略有出入）。
    const loose = needle.slice(0, 12);
    while (node) {
      const txt = (node.textContent || '').replace(/\s+/g, ' ');
      if (txt.includes(needle) || (loose.length >= 4 && txt.includes(loose))) {
        hit = node.parentElement;
        break;
      }
      node = walker.nextNode();
    }
    if (!hit) return false;

    hit.scrollIntoView({ behavior: 'smooth', block: 'center' });
    const prevOutline = hit.style.outline;
    const prevTransition = hit.style.transition;
    hit.style.transition = 'background-color 0.3s, outline 0.3s';
    hit.style.outline = '2px solid #3b82f6';
    hit.style.backgroundColor = 'rgba(59,130,246,0.18)';
    let on = true;
    const blink = setInterval(() => {
      on = !on;
      hit!.style.backgroundColor = on ? 'rgba(59,130,246,0.18)' : 'transparent';
    }, 400);
    setTimeout(() => {
      clearInterval(blink);
      hit!.style.outline = prevOutline;
      hit!.style.backgroundColor = '';
      hit!.style.transition = prevTransition;
    }, 2000);
    return true;
  };

  const goBack = () => {
    if (navIndex > 0) {
      const newIdx = navIndex - 1;
      setNavIndex(newIdx);
      loadProxy(navHistory[newIdx]);
    }
  };

  const goForward = () => {
    if (navIndex < navHistory.length - 1) {
      const newIdx = navIndex + 1;
      setNavIndex(newIdx);
      loadProxy(navHistory[newIdx]);
    }
  };

  const refresh = () => {
    if (navHistory[navIndex]) {
      loadProxy(navHistory[navIndex]);
    }
  };

  const handleTranslateImmersive = useCallback(async () => {
    const iframe = iframeRef.current;
    if (!iframe) return;

    // If already translating or done — cancel/clear
    if (translateState === 'translating') {
      abortRef.current?.abort();
      schedulerRef.current?.stop();
      schedulerRef.current = null;
      setTranslateState('idle');
      setTranslateProgress(null);
      return;
    }
    if (translateState === 'done') {
      schedulerRef.current?.stop();
      schedulerRef.current = null;
      setTranslateState('idle');
      setTranslateProgress(null);
      return;
    }

    // Start translation
    const controller = new AbortController();
    abortRef.current = controller;

    let counted = 0;
    const scheduler = new TranslateScheduler({
      iframe,
      translateFn: async (text: string) => {
        const res = await api.translateText(text, 'zh');
        return res.translated;
      },
      onProgress: (done, total) => {
        counted = total;
        // total 在 Phase1 才算出；为 0 时不显示「0/0」假进度，留给结束后判定空态。
        if (total > 0) setTranslateProgress({ done, total });
        if (total > 0 && done >= total) {
          setTranslateState('done');
        }
      },
      signal: controller.signal,
    });
    schedulerRef.current = scheduler;

    setTranslateState('translating');
    setTranslateProgress(null);
    setTranslateNotice('');

    await scheduler.start();

    if (controller.signal.aborted) return;

    // 无可翻译段落：提示而非静默回到 idle。
    if (counted === 0) {
      setTranslateState('idle');
      setTranslateProgress(null);
      setTranslateNotice('无可翻译内容');
      setTimeout(() => setTranslateNotice(''), 2500);
      return;
    }

    setTranslateState('done');
  }, [translateState]);

  // 监听 App 派发的 'T' 翻译快捷键事件（依赖 handleTranslateImmersive，闭包随 translateState 更新重新绑定）。
  useEffect(() => {
    const handler = () => { handleTranslateImmersive(); };
    window.addEventListener('rss-shortcut-translate', handler);
    return () => window.removeEventListener('rss-shortcut-translate', handler);
  }, [handleTranslateImmersive]);

  const handleCancelTranslate = () => {
    abortRef.current?.abort();
    schedulerRef.current?.stop();
    schedulerRef.current = null;
    setTranslateState('idle');
    setTranslateProgress(null);
  };

  const handleSelectionTranslate = async (text: string) => {
    setSelectionPopup(null);
    try {
      const res = await api.translateText(text, 'zh');
      // Show as a tooltip in iframe；配色复用 .weft-trans（暗色由 DarkReader 基础样式接管），只内联定位。
      const doc = iframeRef.current?.contentDocument;
      if (doc) {
        const tooltip = doc.createElement('div');
        tooltip.className = 'weft-trans';
        tooltip.textContent = res.translated;
        tooltip.style.cssText = 'position:fixed;top:20px;right:20px;max-width:300px;padding:12px;border-radius:8px;z-index:9999;box-shadow:0 4px 12px rgba(0,0,0,0.25);';
        doc.body.appendChild(tooltip);
        setTimeout(() => tooltip.remove(), 5000);
      }
    } catch { /* ignore */ }
  };

  const handleSelectionExplain = async (text: string) => {
    setSelectionPopup(null);
    onToggleAiPanel();
    // The AI panel will handle it via a custom event
    window.dispatchEvent(new CustomEvent('rss-explain-selection', { detail: { text } }));
  };

  const handleSelectionAsk = (text: string) => {
    setSelectionPopup(null);
    onToggleAiPanel();
    window.dispatchEvent(new CustomEvent('rss-ask-selection', { detail: { text } }));
  };

  if (!article) {
    return (
      <div className="flex-[4] min-w-0 flex flex-col items-center justify-center text-muted gap-3">
        <FileText size={36} strokeWidth={1.2} />
        <p className="text-sm">选择一篇文章开始阅读</p>
        <p className="text-xs">支持沉浸式翻译、AI 对话、划词解释</p>
      </div>
    );
  }

  // 选区弹窗水平定位 clamp，避免靠右划词时被裁切。
  const POPUP_WIDTH = 200;
  const containerWidth = containerRef.current?.clientWidth ?? 0;
  const popupLeft = selectionPopup
    ? Math.max(8, Math.min(selectionPopup.x - 60, Math.max(8, containerWidth - POPUP_WIDTH - 8)))
    : 0;

  return (
    <div className="flex-[4] min-w-0 flex flex-col h-full transition-all duration-200">
      {/* Sticky toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border bg-bg-secondary/80 backdrop-blur-sm shrink-0">
        {/* Left */}
        <button
          onClick={onBackToList}
          className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200 transition"
          title="返回列表 [Esc]"
        >
          <ArrowLeft size={16} />
        </button>

        {/* 导航按钮 */}
        <button
          onClick={goBack}
          disabled={navIndex <= 0}
          className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200 transition disabled:opacity-30 disabled:cursor-not-allowed"
          title="后退"
        >
          <ChevronLeft size={16} />
        </button>
        <button
          onClick={goForward}
          disabled={navIndex >= navHistory.length - 1}
          className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200 transition disabled:opacity-30 disabled:cursor-not-allowed"
          title="前进"
        >
          <ChevronRight size={16} />
        </button>
        <button
          onClick={refresh}
          className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200 transition"
          title="刷新"
        >
          <RotateCw size={14} />
        </button>

        <h2 className="text-sm font-medium text-gray-100 truncate flex-1 mx-2">
          {article.title}
        </h2>

        {/* Right toolbar buttons */}
        <button
          onClick={() => onToggleFavorite(article)}
          className={`p-1.5 rounded transition ${article.is_favorite ? 'text-favorite' : 'text-muted hover:text-favorite/60'}`}
          title={article.is_favorite ? '取消收藏' : '收藏'}
        >
          <Heart size={16} fill={article.is_favorite ? 'currentColor' : 'none'} />
        </button>

        <button
          onClick={handleTranslateImmersive}
          className={`p-1.5 rounded transition flex items-center gap-1 ${
            translateState === 'translating' ? 'text-accent' :
            translateState === 'done' ? 'text-green-400' :
            'text-muted hover:text-gray-200'
          }`}
          title={
            translateState === 'translating' ? '点击取消翻译' :
            translateState === 'done' ? '点击清除翻译' :
            '沉浸式翻译 [T]'
          }
        >
          <Languages size={16} />
          {translateState === 'translating' && translateProgress && (
            <span className="text-[10px]">{translateProgress.done}/{translateProgress.total}</span>
          )}
          {translateState === 'done' && (
            <span className="text-[10px]">✓</span>
          )}
        </button>

        <button
          onClick={onToggleAiPanel}
          className="p-1.5 rounded text-muted hover:text-gray-200 transition"
          title="AI 面板 [A]"
        >
          <Bot size={16} />
        </button>

        <button
          onClick={onOpenSettings}
          className="p-1.5 rounded text-muted hover:text-gray-200 transition"
          title="设置"
        >
          <Settings size={16} />
        </button>

        <button
          onClick={onToggleDarkMode}
          className={`p-1.5 rounded transition ${darkMode ? 'text-accent' : 'text-muted hover:text-gray-200'}`}
          title="暗色阅读"
        >
          <Moon size={16} />
        </button>

        <button
          onClick={onToggleZen}
          className={`p-1.5 rounded transition ${isZen ? 'text-accent' : 'text-muted hover:text-gray-200'}`}
          title="全屏禅模式 [F]"
        >
          <Maximize size={16} />
        </button>

        <button
          onClick={onShowShortcuts}
          className="p-1.5 rounded text-muted hover:text-gray-200 transition"
          title="快捷键"
        >
          <HelpCircle size={16} />
        </button>

        {article.link && (
          <a
            href={article.link}
            target="_blank"
            rel="noopener noreferrer"
            className="p-1.5 rounded text-muted hover:text-gray-200 transition"
            title="在浏览器打开"
          >
            <ExternalLink size={16} />
          </a>
        )}
      </div>

      {/* Translate progress */}
      {translateState === 'translating' && (
        <div className="shrink-0">
          <div className="h-0.5 bg-bg-tertiary overflow-hidden">
            {translateProgress && translateProgress.total > 0 ? (
              <div
                className="h-full bg-accent transition-all duration-300"
                style={{ width: `${(translateProgress.done / translateProgress.total) * 100}%` }}
              />
            ) : (
              // total 尚未算出：indeterminate 动画，避免「卡在 0%」的观感
              <div className="h-full w-1/3 bg-accent animate-pulse" />
            )}
          </div>
          <div className="px-4 py-1.5 bg-bg-tertiary border-b border-border flex items-center gap-2">
            <Loader2 size={12} className="animate-spin text-accent" />
            <span className="text-xs text-muted">
              {translateProgress && translateProgress.total > 0
                ? `翻译中 ${translateProgress.done}/${translateProgress.total} 段`
                : '准备翻译...'}
            </span>
            <button onClick={handleCancelTranslate} className="text-xs text-red-400 hover:text-red-300 ml-2">
              取消
            </button>
          </div>
        </div>
      )}

      {/* 翻译空态提示 */}
      {translateNotice && (
        <div className="shrink-0 px-4 py-1.5 bg-bg-tertiary border-b border-border text-xs text-muted">
          {translateNotice}
        </div>
      )}

      {/* Content area with iframe */}
      <div ref={containerRef} className="flex-1 overflow-hidden relative">
        {frameMode === 'loading' ? (
          <div className="flex items-center justify-center h-full text-muted text-xs gap-2">
            <div className="animate-spin w-4 h-4 border-2 border-accent border-t-transparent rounded-full" />
            正在加载...
          </div>
        ) : frameMode === 'proxy' ? (
          <iframe
            ref={iframeRef}
            srcDoc={proxiedHtml}
            title={article.title}
            onLoad={handleIframeLoad}
            className="w-full h-full border-none bg-white"
            sandbox="allow-same-origin allow-scripts allow-popups allow-forms"
          />
        ) : frameMode === 'direct' ? (
          article.link ? (
            <iframe
              ref={iframeRef}
              src={article.link}
              title={article.title}
              onLoad={handleIframeLoad}
              className="w-full h-full border-none bg-white"
              sandbox="allow-same-origin allow-scripts allow-popups allow-forms"
            />
          ) : (
            <div className={`p-6 overflow-y-auto h-full ${darkMode ? 'bg-[#1a1a2e]' : 'bg-white'}`}>
              {article.content && (
                <div
                  dangerouslySetInnerHTML={{ __html: article.content }}
                  className={`max-w-[720px] mx-auto leading-[1.8] ${darkMode ? 'text-gray-200' : 'text-gray-900'}`}
                  style={{ fontSize: `${fontSize}px` }}
                />
              )}
            </div>
          )
        ) : (
          <div className={`flex flex-col items-center justify-center h-full text-muted gap-3 p-4 ${darkMode ? 'bg-[#1a1a2e]' : ''}`}>
            <p className="text-sm">页面加载失败</p>
            {proxyError && <p className="text-xs text-red-400 max-w-md text-center">{proxyError}</p>}
            {article.link && (
              <div className="flex gap-2">
                <button
                  onClick={() => article.link && loadProxy(article.link)}
                  className="px-3 py-1.5 text-xs bg-accent hover:bg-accent-hover text-white rounded-md transition"
                >
                  重试代理加载
                </button>
                <a href={article.link} target="_blank" rel="noopener noreferrer" className="px-3 py-1.5 text-xs bg-bg-tertiary hover:bg-bg-hover text-gray-300 rounded-md flex items-center gap-1 transition">
                  <ExternalLink size={12} /> 在浏览器打开
                </a>
              </div>
            )}
            {article.content && (
              <div className={`mt-4 p-4 rounded-lg max-h-96 overflow-y-auto w-full max-w-[720px] ${darkMode ? 'bg-[#23233a]' : 'bg-white'}`}>
                <div
                  dangerouslySetInnerHTML={{ __html: article.content }}
                  className={`text-sm leading-relaxed ${darkMode ? 'text-gray-200' : 'text-gray-800'}`}
                />
              </div>
            )}
          </div>
        )}

        {/* Selection popup */}
        {selectionPopup && (
          <div
            className="absolute z-50 flex gap-1 bg-bg-secondary border border-border rounded-lg p-1 shadow-xl"
            style={{ left: popupLeft, top: Math.max(0, selectionPopup.y) }}
          >
            <button
              onClick={() => handleSelectionTranslate(selectionPopup.text)}
              className="px-2 py-1 text-xs text-gray-300 hover:bg-bg-hover rounded transition"
            >
              翻译
            </button>
            <button
              onClick={() => handleSelectionExplain(selectionPopup.text)}
              className="px-2 py-1 text-xs text-gray-300 hover:bg-bg-hover rounded transition"
            >
              解释
            </button>
            <button
              onClick={() => handleSelectionAsk(selectionPopup.text)}
              className="px-2 py-1 text-xs text-gray-300 hover:bg-bg-hover rounded transition"
            >
              问AI
            </button>
            <button
              onClick={() => setSelectionPopup(null)}
              className="px-1 py-1 text-xs text-muted hover:text-gray-200 rounded transition"
            >
              <X size={12} />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
