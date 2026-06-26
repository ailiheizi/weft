import { useState, useEffect, useCallback } from 'react';
import { api, hasToken } from './api';
import type { Feed, Article } from './api';
import { FeedsPane } from './components/FeedsPane';
import type { LayoutMode } from './components/FeedsPane';
import { ArticlesPane } from './components/ArticlesPane';
import type { ViewMode } from './components/ArticlesPane';
import { ReaderPane } from './components/ReaderPane';
import { AiPanel } from './components/AiReadingPanel';
import { TokenInput } from './components/TokenInput';
import { ShortcutOverlay } from './components/ShortcutOverlay';
import './App.css';

export default function App() {
  const [tokenReady, setTokenReady] = useState(hasToken());
  const [feeds, setFeeds] = useState<Feed[]>([]);
  const [selectedFeedId, setSelectedFeedId] = useState<number | null>(null);
  const [articles, setArticles] = useState<Article[]>([]);
  const [selectedArticle, setSelectedArticle] = useState<Article | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('articles');
  const [unreadOnly, setUnreadOnly] = useState(false);
  const [loading, setLoading] = useState({ feeds: false, articles: false });

  // Layout: full | reading | zen
  const [layoutMode, setLayoutMode] = useState<LayoutMode>('full');
  // AI panel
  const [aiPanelOpen, setAiPanelOpen] = useState(false);
  const [aiPanelTab, setAiPanelTab] = useState<'chat' | 'sections' | 'settings'>('chat');
  // Shortcuts overlay
  const [showShortcuts, setShowShortcuts] = useState(false);

  // 阅读偏好：暗色 + 字号，持久化到 localStorage
  const [darkMode, setDarkMode] = useState<boolean>(() => localStorage.getItem('weft-dark-mode') === '1');
  const [fontSize, setFontSize] = useState<number>(() => {
    const v = Number(localStorage.getItem('weft-font-size'));
    return v >= 12 && v <= 24 ? v : 16;
  });
  useEffect(() => { localStorage.setItem('weft-dark-mode', darkMode ? '1' : '0'); }, [darkMode]);
  useEffect(() => { localStorage.setItem('weft-font-size', String(fontSize)); }, [fontSize]);


  const loadFeeds = useCallback(async () => {
    setLoading(l => ({ ...l, feeds: true }));
    try {
      const res = await api.listFeeds();
      setFeeds(res.feeds || []);
    } catch (e) { console.error('loadFeeds:', e); }
    setLoading(l => ({ ...l, feeds: false }));
  }, []);

  const loadArticles = useCallback(async () => {
    setLoading(l => ({ ...l, articles: true }));
    try {
      const opts: Record<string, unknown> = { limit: 100 };
      if (viewMode === 'favorites') {
        opts.favorites_only = true;
        opts.limit = 200;
      } else if (viewMode === 'articles') {
        if (selectedFeedId) opts.feed_id = selectedFeedId;
        if (unreadOnly) opts.unread_only = true;
      }
      const res = await api.listArticles(opts as Parameters<typeof api.listArticles>[0]);
      setArticles(res.articles || []);
    } catch (e) { console.error('loadArticles:', e); }
    setLoading(l => ({ ...l, articles: false }));
  }, [selectedFeedId, unreadOnly, viewMode]);

  useEffect(() => { loadFeeds(); }, [loadFeeds]);
  useEffect(() => {
    if (viewMode !== 'recommend') loadArticles();
  }, [loadArticles, viewMode]);

  // Select article (no auto layout switch — use zen button for fullscreen)
  const handleSelectArticle = async (article: Article) => {
    setSelectedArticle(article);
    if (!article.is_read) {
      try {
        await api.markRead(article.id);
        setArticles(prev => prev.map(a => a.id === article.id ? { ...a, is_read: 1 } : a));
        loadFeeds();
      } catch (e) { console.error(e); }
    }
  };

  const handleToggleFavorite = async (article: Article) => {
    const next = !article.is_favorite;
    try {
      await api.markFavorite(article.id, next);
      const updated = { ...article, is_favorite: next ? 1 : 0 };
      setArticles(prev => prev.map(a => a.id === article.id ? updated : a));
      if (selectedArticle?.id === article.id) setSelectedArticle(updated);
    } catch (e) { console.error(e); }
  };

  const handleRefreshAll = async () => {
    try {
      await api.refreshAll();
      await loadFeeds();
      await loadArticles();
    } catch (e) { console.error(e); }
  };

  const handleAddFeed = async (url: string) => {
    await api.addFeed(url);
    loadFeeds();
  };

  const handleRemoveFeed = async (feedId: number) => {
    await api.removeFeed(feedId);
    setFeeds(prev => prev.filter(f => f.id !== feedId));
    if (selectedFeedId === feedId) setSelectedFeedId(null);
    loadArticles();
  };

  const handleBackToList = () => {
    setLayoutMode('full');
  };

  const handleToggleZen = () => {
    setLayoutMode(prev => prev === 'zen' ? 'full' : 'zen');
  };

  const handleOpenSettings = () => {
    setAiPanelTab('settings');
    setAiPanelOpen(true);
  };

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Don't handle if typing in an input/textarea
      const target = e.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT') return;

      switch (e.key) {
        case 'Escape':
          if (showShortcuts) { setShowShortcuts(false); return; }
          if (aiPanelOpen) { setAiPanelOpen(false); return; }
          if (layoutMode === 'zen') { setLayoutMode('full'); return; }
          break;
        case 'f':
        case 'F':
          if (!e.ctrlKey && !e.metaKey) {
            e.preventDefault();
            handleToggleZen();
          }
          break;
        case 't':
        case 'T':
          if (!e.ctrlKey && !e.metaKey) {
            // Translate shortcut is handled by ReaderPane via its own button click
            // We dispatch a custom event
            window.dispatchEvent(new CustomEvent('rss-shortcut-translate'));
          }
          break;
        case 'a':
        case 'A':
          if (!e.ctrlKey && !e.metaKey) {
            setAiPanelOpen(prev => !prev);
          }
          break;
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [layoutMode, aiPanelOpen, showShortcuts]);

  if (!tokenReady) {
    return <TokenInput onTokenSet={() => setTokenReady(true)} />;
  }

  return (
    <div className="flex h-full overflow-hidden">
      <FeedsPane
        feeds={feeds}
        selectedFeedId={selectedFeedId}
        loading={loading.feeds}
        layoutMode={layoutMode}
        onSelect={setSelectedFeedId}
        onAdd={handleAddFeed}
        onRemove={handleRemoveFeed}
        onRefreshAll={handleRefreshAll}
        onOpenSettings={handleOpenSettings}
      />
      {layoutMode !== 'zen' && <div className="w-px bg-border" />}
      <ArticlesPane
        articles={articles}
        selectedArticle={selectedArticle}
        viewMode={viewMode}
        unreadOnly={unreadOnly}
        loading={loading.articles}
        visible={layoutMode !== 'zen'}
        onSelectArticle={handleSelectArticle}
        onToggleFavorite={handleToggleFavorite}
        onViewModeChange={setViewMode}
        onUnreadOnlyChange={setUnreadOnly}
        onRefresh={loadArticles}
        selectedFeedId={selectedFeedId}
      />
      {layoutMode === 'full' && <div className="w-px bg-border" />}
      <ReaderPane
        article={selectedArticle}
        onToggleFavorite={handleToggleFavorite}
        onBackToList={handleBackToList}
        onToggleAiPanel={() => setAiPanelOpen(prev => !prev)}
        onOpenSettings={handleOpenSettings}
        onToggleZen={handleToggleZen}
        onShowShortcuts={() => setShowShortcuts(true)}
        isZen={layoutMode === 'zen'}
        darkMode={darkMode}
        onToggleDarkMode={() => setDarkMode(prev => !prev)}
        fontSize={fontSize}
      />

      {/* AI side panel (overlay from right) */}
      <AiPanel
        articleId={selectedArticle?.id ?? null}
        open={aiPanelOpen}
        onClose={() => setAiPanelOpen(false)}
        initialTab={aiPanelTab}
        fontSize={fontSize}
        onFontSizeChange={setFontSize}
      />

      {/* Shortcuts overlay */}
      <ShortcutOverlay open={showShortcuts} onClose={() => setShowShortcuts(false)} />
    </div>
  );
}
