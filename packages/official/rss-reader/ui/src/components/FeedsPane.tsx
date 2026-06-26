import { useState } from 'react';
import { Plus, RefreshCw, X, Rss, Settings } from 'lucide-react';
import type { Feed } from '../api';

export type LayoutMode = 'full' | 'reading' | 'zen';

interface Props {
  feeds: Feed[];
  selectedFeedId: number | null;
  loading: boolean;
  layoutMode: LayoutMode;
  onSelect: (id: number | null) => void;
  onAdd: (url: string) => void;
  onRemove: (id: number) => void;
  onRefreshAll: () => void;
  onOpenSettings: () => void;
}

export function FeedsPane({
  feeds, selectedFeedId, loading, layoutMode,
  onSelect, onAdd, onRemove, onRefreshAll, onOpenSettings,
}: Props) {
  const [showAdd, setShowAdd] = useState(false);
  const [addUrl, setAddUrl] = useState('');
  const [adding, setAdding] = useState(false);
  const [hovered, setHovered] = useState(false);

  const totalUnread = feeds.reduce((sum, f) => sum + (f.unread || 0), 0);
  const collapsed = layoutMode === 'reading' || layoutMode === 'zen';

  const handleAdd = async () => {
    if (!addUrl.trim()) return;
    setAdding(true);
    try {
      await onAdd(addUrl.trim());
      setAddUrl('');
      setShowAdd(false);
    } catch (e) { console.error(e); }
    setAdding(false);
  };

  const feedListContent = (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-3 border-b border-border">
        <Rss size={16} className="text-accent" />
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted flex-1">订阅源</h3>
        <button onClick={onRefreshAll} className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200 transition" title="全部刷新">
          <RefreshCw size={14} />
        </button>
        <button onClick={() => setShowAdd(true)} className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200 transition" title="添加订阅">
          <Plus size={14} />
        </button>
      </div>

      {/* Feed list */}
      <div className="flex-1 overflow-y-auto py-1">
        <button
          onClick={() => onSelect(null)}
          className={`w-full flex items-center gap-2 px-4 py-2 text-left border-l-2 transition ${
            selectedFeedId === null ? 'border-accent bg-bg-selected text-gray-100' : 'border-transparent hover:bg-bg-hover text-gray-300'
          }`}
        >
          <span className="flex-1 text-sm truncate">全部文章</span>
          {totalUnread > 0 && (
            <span className="bg-accent text-white text-xs px-1.5 py-0.5 rounded-full font-medium min-w-5 text-center">{totalUnread}</span>
          )}
        </button>

        {loading && feeds.length === 0 && (
          <div className="px-4 py-6 text-center text-muted text-xs">加载中...</div>
        )}

        {feeds.map(feed => (
          <button
            key={feed.id}
            onClick={() => onSelect(feed.id)}
            className={`w-full flex items-center gap-2 px-4 py-2 text-left border-l-2 transition group ${
              selectedFeedId === feed.id ? 'border-accent bg-bg-selected text-gray-100' : 'border-transparent hover:bg-bg-hover text-gray-300'
            }`}
          >
            <span className="flex-1 text-sm truncate">{feed.title || feed.url}</span>
            {feed.unread > 0 && (
              <span className="bg-accent/20 text-accent text-xs px-1.5 py-0.5 rounded-full font-medium min-w-5 text-center">{feed.unread}</span>
            )}
            <button
              onClick={e => { e.stopPropagation(); onRemove(feed.id); }}
              className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-red-500/20 text-muted hover:text-red-400 transition"
              title="删除"
            >
              <X size={12} />
            </button>
          </button>
        ))}
      </div>

      {/* Settings at bottom */}
      <div className="border-t border-border px-3 py-2">
        <button
          onClick={onOpenSettings}
          className="w-full flex items-center gap-2 px-2 py-1.5 text-sm text-muted hover:text-gray-200 hover:bg-bg-hover rounded transition"
        >
          <Settings size={14} />
          <span>设置</span>
        </button>
      </div>

      {/* Add dialog */}
      {showAdd && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={() => setShowAdd(false)}>
          <div className="bg-bg-secondary border border-border rounded-lg p-5 w-80 shadow-xl" onClick={e => e.stopPropagation()}>
            <h3 className="text-sm font-semibold mb-3">添加订阅源</h3>
            <input
              type="url"
              placeholder="输入 RSS/Atom Feed URL"
              value={addUrl}
              onChange={e => setAddUrl(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleAdd()}
              autoFocus
              className="w-full px-3 py-2 bg-bg-primary border border-border rounded-md text-sm text-gray-200 placeholder:text-muted focus:outline-none focus:ring-1 focus:ring-accent"
            />
            <div className="flex justify-end gap-2 mt-4">
              <button onClick={() => setShowAdd(false)} className="px-3 py-1.5 text-xs rounded-md hover:bg-bg-hover text-muted">取消</button>
              <button
                onClick={handleAdd}
                disabled={adding || !addUrl.trim()}
                className="px-3 py-1.5 text-xs rounded-md bg-accent hover:bg-accent-hover text-white font-medium disabled:opacity-40"
              >
                {adding ? '添加中...' : '添加'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );

  // Collapsed mode: thin icon bar + hover flyout
  if (collapsed) {
    return (
      <div
        className="relative h-full flex-shrink-0 transition-all duration-200"
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        {/* Thin icon bar */}
        <div className="w-12 h-full flex flex-col items-center py-3 bg-bg-secondary border-r border-border gap-2">
          <Rss size={18} className="text-accent" />
          {totalUnread > 0 && (
            <span className="text-[10px] bg-accent/20 text-accent px-1.5 py-0.5 rounded-full">
              {totalUnread}
            </span>
          )}
        </div>

        {/* Flyout on hover */}
        {hovered && (
          <div className="absolute top-0 left-0 z-50 w-64 h-full bg-bg-secondary shadow-2xl border-r border-border">
            {feedListContent}
          </div>
        )}
      </div>
    );
  }

  // Full mode
  return (
    <div className="w-64 min-w-48 flex-shrink-0 flex flex-col bg-bg-secondary transition-all duration-200">
      {feedListContent}
    </div>
  );
}
