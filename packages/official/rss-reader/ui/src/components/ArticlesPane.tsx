import { useState } from 'react';
import { RefreshCw, Heart, Sparkles, FileText, BookOpen } from 'lucide-react';
import { api } from '../api';
import type { Article, Recommendation, RecommendResult } from '../api';

export type ViewMode = 'articles' | 'favorites' | 'recommend';

interface Props {
  articles: Article[];
  selectedArticle: Article | null;
  viewMode: ViewMode;
  unreadOnly: boolean;
  loading: boolean;
  visible: boolean;
  onSelectArticle: (a: Article) => void;
  onToggleFavorite: (a: Article) => void;
  onViewModeChange: (m: ViewMode) => void;
  onUnreadOnlyChange: (v: boolean) => void;
  onRefresh: () => void;
  selectedFeedId: number | null;
}

export function ArticlesPane({
  articles, selectedArticle, viewMode, unreadOnly, loading, visible,
  onSelectArticle, onToggleFavorite, onViewModeChange, onUnreadOnlyChange, onRefresh, selectedFeedId
}: Props) {
  const [recResult, setRecResult] = useState<RecommendResult | null>(null);
  const [recLoading, setRecLoading] = useState(false);

  const loadRecommend = async (force = false) => {
    setRecLoading(true);
    try {
      const res = await api.recommend({
        feed_id: selectedFeedId ?? undefined,
        limit: 15,
        force_refresh: force,
      });
      setRecResult(res);
    } catch (e) { console.error(e); }
    setRecLoading(false);
  };

  const handleViewChange = (m: ViewMode) => {
    onViewModeChange(m);
    if (m === 'recommend' && !recResult) loadRecommend();
  };

  const tabs: { key: ViewMode; label: string; icon: React.ReactNode }[] = [
    { key: 'articles', label: '文章', icon: <FileText size={13} /> },
    { key: 'favorites', label: '收藏', icon: <Heart size={13} /> },
    { key: 'recommend', label: '推荐', icon: <Sparkles size={13} /> },
  ];

  if (!visible) return null;

  return (
    <div className="flex-[3] min-w-72 flex flex-col bg-bg-primary transition-all duration-200">
      {/* Tab bar + toolbar */}
      <div className="flex items-center gap-1 px-3 py-2 border-b border-border">
        <div className="flex bg-bg-tertiary rounded-md p-0.5">
          {tabs.map(t => (
            <button
              key={t.key}
              onClick={() => handleViewChange(t.key)}
              className={`flex items-center gap-1 px-2.5 py-1 text-xs rounded transition ${
                viewMode === t.key ? 'bg-accent text-white font-medium shadow-sm' : 'text-muted hover:text-gray-300'
              }`}
            >
              {t.icon}{t.label}
            </button>
          ))}
        </div>
        <div className="flex-1" />
        {viewMode === 'articles' && (
          <label className="flex items-center gap-1.5 text-xs text-muted cursor-pointer">
            <input
              type="checkbox"
              checked={unreadOnly}
              onChange={e => onUnreadOnlyChange(e.target.checked)}
              className="accent-accent w-3.5 h-3.5"
            />
            仅未读
          </label>
        )}
        {viewMode === 'favorites' && (
          <button onClick={onRefresh} className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200" title="刷新">
            <RefreshCw size={14} />
          </button>
        )}
        {viewMode === 'recommend' && (
          <button onClick={() => loadRecommend(true)} className="p-1.5 rounded hover:bg-bg-hover text-muted hover:text-gray-200" title="重新推荐">
            <RefreshCw size={14} />
          </button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {viewMode === 'recommend' ? (
          <RecommendList result={recResult} loading={recLoading} onSelect={onSelectArticle} />
        ) : (
          <ArticleList
            articles={articles}
            selectedId={selectedArticle?.id ?? null}
            loading={loading}
            viewMode={viewMode}
            onSelect={onSelectArticle}
            onToggleFavorite={onToggleFavorite}
          />
        )}
      </div>
    </div>
  );
}

function ArticleList({ articles, selectedId, loading, viewMode, onSelect, onToggleFavorite }: {
  articles: Article[];
  selectedId: number | null;
  loading: boolean;
  viewMode: ViewMode;
  onSelect: (a: Article) => void;
  onToggleFavorite: (a: Article) => void;
}) {
  if (loading && articles.length === 0) {
    return <div className="flex items-center justify-center h-32 text-muted text-xs">加载中...</div>;
  }
  if (articles.length === 0) {
    const msg = viewMode === 'favorites'
      ? '暂无收藏，点文章右侧 ♥ 收藏感兴趣的内容'
      : '暂无文章，试试刷新订阅源';
    return (
      <div className="flex flex-col items-center justify-center h-48 text-muted gap-2">
        <BookOpen size={28} strokeWidth={1.5} />
        <p className="text-xs">{msg}</p>
      </div>
    );
  }

  return (
    <div>
      {viewMode === 'favorites' && (
        <div className="px-4 py-2 text-xs text-muted border-b border-border">共 {articles.length} 篇收藏</div>
      )}
      {articles.map(a => (
        <button
          key={a.id}
          onClick={() => onSelect(a)}
          className={`w-full flex items-start gap-2.5 px-4 py-3 text-left border-l-2 transition ${
            selectedId === a.id ? 'border-accent bg-bg-selected' : 'border-transparent hover:bg-bg-hover'
          }`}
        >
          {/* Unread dot */}
          <div className={`w-2 h-2 rounded-full mt-1.5 shrink-0 ${a.is_read ? 'bg-transparent' : 'bg-accent'}`} />
          {/* Body */}
          <div className="flex-1 min-w-0">
            <div className={`text-sm leading-snug line-clamp-2 ${!a.is_read ? 'font-medium text-gray-100' : 'text-gray-400'}`}>
              {a.title || '(无标题)'}
            </div>
            {a.published_at && (
              <div className="text-xs text-muted mt-0.5">{a.published_at.slice(0, 16)}</div>
            )}
            {(a.summary || a.content) && (
              <div className="text-xs text-gray-500 mt-1 line-clamp-2">
                {(a.summary || a.content || '').replace(/<[^>]*>/g, '').slice(0, 100)}
              </div>
            )}
          </div>
          {/* Favorite */}
          <button
            onClick={e => { e.stopPropagation(); onToggleFavorite(a); }}
            className={`shrink-0 p-1 rounded transition ${a.is_favorite ? 'text-favorite' : 'text-muted hover:text-favorite/60'}`}
            title={a.is_favorite ? '取消收藏' : '收藏'}
          >
            <Heart size={14} fill={a.is_favorite ? 'currentColor' : 'none'} />
          </button>
        </button>
      ))}
    </div>
  );
}

function RecommendList({ result, loading, onSelect }: {
  result: RecommendResult | null;
  loading: boolean;
  onSelect: (a: Article) => void;
}) {
  if (loading) {
    return <div className="flex items-center justify-center h-32 text-muted text-xs gap-2"><Sparkles size={14} className="animate-pulse" />AI 正在为你精选...</div>;
  }
  if (!result) {
    return (
      <div className="flex flex-col items-center justify-center h-48 text-muted gap-2">
        <Sparkles size={28} strokeWidth={1.5} />
        <p className="text-xs">点击上方推荐按钮开始</p>
      </div>
    );
  }

  const { recommendations, mode, profile_size } = result;

  const banner = mode === 'cold_start'
    ? `多读几篇、点收藏，推荐会越来越懂你（已读 ${profile_size} 篇）`
    : mode === 'l2_fallback'
      ? '按相关度排序（AI 精排暂不可用）'
      : '根据你读过和喜欢的文章，AI 为你精选';

  if (recommendations.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-48 text-muted gap-2">
        <BookOpen size={28} strokeWidth={1.5} />
        <p className="text-xs">暂无推荐，多读几篇再试</p>
      </div>
    );
  }

  return (
    <div>
      <div className="px-4 py-2.5 text-xs text-muted border-b border-border">{banner}</div>
      {recommendations.map((rec: Recommendation) => (
        <button
          key={rec.article_id}
          onClick={() => onSelect({ id: rec.article_id, title: rec.title, link: rec.link } as Article)}
          className="w-full text-left px-4 py-3 border-b border-border/50 hover:bg-bg-hover transition"
        >
          <div className="flex items-start gap-2">
            <span className="shrink-0 bg-accent text-white text-xs px-1.5 py-0.5 rounded font-semibold">{Math.round(rec.score)}</span>
            <span className="text-sm text-gray-200 line-clamp-2">{rec.title}</span>
          </div>
          {rec.rationale_zh && (
            <p className="text-xs text-gray-400 mt-1.5 ml-7">{rec.rationale_zh}</p>
          )}
          {rec.matched_keywords?.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-1.5 ml-7">
              {rec.matched_keywords.map(k => (
                <span key={k} className="bg-accent/10 text-accent text-xs px-1.5 py-0.5 rounded">{k}</span>
              ))}
            </div>
          )}
          {(rec.aversion || !rec.grounded) && (
            <div className="text-xs text-muted mt-1 ml-7">
              {rec.aversion && <span>{rec.aversion}</span>}
              {!rec.grounded && <span className="ml-2 text-yellow-600">低置信</span>}
            </div>
          )}
        </button>
      ))}
    </div>
  );
}
