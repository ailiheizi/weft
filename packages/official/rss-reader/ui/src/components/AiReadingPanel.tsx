import { useState, useEffect, useRef } from 'react';
import {
  MessageSquare, BookOpen, Settings, Send, Loader2, X, Anchor, Globe,
} from 'lucide-react';
import { api, listProviders } from '../api';
import type { Provider } from '../api';

interface Props {
  articleId: number | null;
  open: boolean;
  onClose: () => void;
  initialTab?: 'chat' | 'sections' | 'settings';
  fontSize: number;
  onFontSizeChange: (size: number) => void;
}

interface ChatMsg {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

interface Section {
  index: number;
  summary: string;
  keywords: string[];
  start_text: string;
}

export function AiPanel({ articleId, open, onClose, initialTab, fontSize, onFontSizeChange }: Props) {
  const [tab, setTab] = useState<'chat' | 'sections' | 'settings'>(initialTab || 'chat');
  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [searching, setSearching] = useState(false);
  const [sections, setSections] = useState<{ overview: string; sections: Section[] } | null>(null);
  const [sectionsLoading, setSectionsLoading] = useState(false);
  const [sectionNotice, setSectionNotice] = useState('');

  // Settings tab
  const [providers, setProviders] = useState<Provider[]>([]);
  const [selectedProvider, setSelectedProvider] = useState('');
  const [selectedModel, setSelectedModel] = useState('');
  const [settingsLoading, setSettingsLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [settingsMsg, setSettingsMsg] = useState('');

  // Translate settings
  const [translateProvider, setTranslateProvider] = useState('mymemory');
  const [translateTarget, setTranslateTarget] = useState('zh');
  const [translateApiKey, setTranslateApiKey] = useState('');
  const [translateRegion, setTranslateRegion] = useState('');

  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Listen for selection events from ReaderPane
  useEffect(() => {
    const handleExplain = (e: Event) => {
      const { text } = (e as CustomEvent).detail;
      setTab('chat');
      doExplain(text);
    };
    const handleAsk = (e: Event) => {
      const { text } = (e as CustomEvent).detail;
      setTab('chat');
      setInput(`关于这段内容: "${text.slice(0, 100)}"\n我的问题是: `);
    };
    window.addEventListener('rss-explain-selection', handleExplain);
    window.addEventListener('rss-ask-selection', handleAsk);
    return () => {
      window.removeEventListener('rss-explain-selection', handleExplain);
      window.removeEventListener('rss-ask-selection', handleAsk);
    };
  }, []);

  useEffect(() => {
    if (initialTab) setTab(initialTab);
  }, [initialTab]);

  // Scroll to bottom of messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Reset on article change
  useEffect(() => {
    setMessages([]);
    setSections(null);
    setSectionNotice('');
  }, [articleId]);

  const doExplain = async (text: string) => {
    setMessages(prev => [...prev, { role: 'user', content: `请解释: "${text.slice(0, 200)}"` }]);
    setSending(true);
    try {
      const res = await api.explainSelection(text);
      setMessages(prev => [...prev, { role: 'assistant', content: res.explanation }]);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'error';
      setMessages(prev => [...prev, { role: 'assistant', content: `解释失败: ${msg}` }]);
    }
    setSending(false);
  };

  // 把 tool-web 返回的搜索结果格式化为可读文本。结构未知时尽量兼容多种字段。
  const formatSearchResults = (raw: any): string => {
    let list: any[] = [];
    if (Array.isArray(raw)) list = raw;
    else if (raw && Array.isArray(raw.results)) list = raw.results;
    else if (raw && raw.results && Array.isArray(raw.results.results)) list = raw.results.results;
    else if (raw && Array.isArray(raw.items)) list = raw.items;

    if (list.length === 0) {
      // 没识别出列表结构：直接把内容压成字符串展示。
      const txt = typeof raw === 'string' ? raw : JSON.stringify(raw, null, 2);
      return txt.slice(0, 2000);
    }
    return list.slice(0, 6).map((r, i) => {
      const title = r.title || r.name || r.url || `结果 ${i + 1}`;
      const url = r.url || r.link || '';
      const snippet = r.snippet || r.text || r.content || r.description || '';
      return `${i + 1}. ${title}\n${url ? url + '\n' : ''}${String(snippet).slice(0, 300)}`;
    }).join('\n\n');
  };

  // 执行联网搜索：结果作为 system 消息插入对话，再让 AI 基于结果+文章作答。
  const doWebSearch = async (query: string, alsoAsk: boolean) => {
    setSearching(true);
    setMessages(prev => [...prev, { role: 'user', content: `🔍 搜索: ${query}` }]);
    try {
      const res = await api.webSearch(query);
      const formatted = formatSearchResults(res.results);
      setMessages(prev => [...prev, { role: 'system', content: `联网搜索结果:\n\n${formatted}` }]);

      if (alsoAsk && articleId) {
        setSending(true);
        const followup = `请结合下面的联网搜索结果回答我的问题「${query}」，并在需要时关联本文内容:\n\n${formatted}`;
        const reply = await api.chatWithArticle(articleId, followup);
        setMessages(prev => [...prev, { role: 'assistant', content: reply.reply }]);
        setSending(false);
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'error';
      setMessages(prev => [...prev, { role: 'assistant', content: `搜索失败: ${msg}` }]);
      setSending(false);
    }
    setSearching(false);
  };

  const handleSend = async () => {
    if (!input.trim() || sending || searching) return;
    const msg = input.trim();
    setInput('');
    // 自动检测搜索意图：以「搜/查一下/search」等开头时，先联网搜索再让 AI 结合作答。
    const searchMatch = msg.match(/^\s*(?:搜索?一?下?|查一?下?|search)[:：\s]+(.+)/i);
    if (searchMatch && searchMatch[1].trim()) {
      await doWebSearch(searchMatch[1].trim(), true);
      return;
    }
    if (!articleId) return;
    setMessages(prev => [...prev, { role: 'user', content: msg }]);
    setSending(true);
    try {
      const res = await api.chatWithArticle(articleId, msg);
      setMessages(prev => [...prev, { role: 'assistant', content: res.reply }]);
    } catch (e: unknown) {
      const msg2 = e instanceof Error ? e.message : 'error';
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${msg2}` }]);
    }
    setSending(false);
  };

  // 点输入框旁的搜索按钮：把当前输入当作搜索词，仅展示结果（不强制追问）。
  const handleSearchClick = async () => {
    const q = input.trim();
    if (!q || sending || searching) return;
    setInput('');
    await doWebSearch(q, false);
  };

  // 点分段大纲某项：通知 ReaderPane 跳到原文。命中失败时给提示。
  const handleSectionJump = (sec: Section) => {
    if (!sec.start_text) {
      setSectionNotice(`第 ${sec.index} 段缺少定位锚点`);
      setTimeout(() => setSectionNotice(''), 2500);
      return;
    }
    const onResult = (e: Event) => {
      const d = (e as CustomEvent).detail || {};
      if (d.index === sec.index) {
        if (!d.found) {
          setSectionNotice(`第 ${sec.index} 段在原文中未找到对应位置`);
          setTimeout(() => setSectionNotice(''), 2500);
        }
        window.removeEventListener('rss-scroll-result', onResult);
      }
    };
    window.addEventListener('rss-scroll-result', onResult);
    window.dispatchEvent(new CustomEvent('rss-scroll-to-text', { detail: { index: sec.index, text: sec.start_text } }));
  };

  const loadSections = async () => {
    if (!articleId) return;
    setSectionsLoading(true);
    try {
      const res = await api.analyzeSections(articleId);
      setSections(res.analysis || JSON.parse(res.analysis_raw || '{}'));
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'error';
      setSections({ overview: `分析失败: ${msg}`, sections: [] });
    }
    setSectionsLoading(false);
  };

  const loadSettings = async () => {
    setSettingsLoading(true);
    try {
      const [provs, config, transConfig] = await Promise.all([
        listProviders().catch(() => [] as Provider[]),
        api.getAiConfig().catch(() => ({ model: '', base_url: '', has_api_key: false, api_key_masked: '' })),
        api.getTranslateConfig().catch(() => ({ provider: 'mymemory', target: 'zh', api_key: '', region: '' })),
      ]);
      setProviders(provs);
      const savedModel = config.model || '';
      const matchedProv = provs.find(p => p.models.includes(savedModel));
      if (matchedProv) {
        setSelectedProvider(matchedProv.name);
        setSelectedModel(savedModel);
      } else if (provs.length > 0) {
        setSelectedProvider(provs[0].name);
        setSelectedModel(provs[0].models[0] || '');
      }
      // Load translate settings
      setTranslateProvider(transConfig.provider || 'mymemory');
      setTranslateTarget(transConfig.target || 'zh');
      setTranslateApiKey(transConfig.api_key || '');
      setTranslateRegion(transConfig.region || '');
    } catch { /* ignore */ }
    setSettingsLoading(false);
  };

  const handleProviderChange = (name: string) => {
    setSelectedProvider(name);
    const prov = providers.find(p => p.name === name);
    setSelectedModel(prov?.models[0] || '');
  };

  const handleSaveSettings = async () => {
    setSaving(true);
    setSettingsMsg('');
    try {
      await Promise.all([
        api.setAiConfig({ model: selectedModel, provider: selectedProvider }),
        api.setTranslateConfig({
          provider: translateProvider,
          target: translateTarget,
          api_key: translateApiKey || undefined,
          region: translateRegion || undefined,
        }),
      ]);
      setSettingsMsg('已保存');
      setTimeout(() => setSettingsMsg(''), 2000);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'error';
      setSettingsMsg(`保存失败: ${msg}`);
    }
    setSaving(false);
  };

  const currentModels = providers.find(p => p.name === selectedProvider)?.models || [];

  const tabs: { key: typeof tab; label: string; icon: React.ReactNode }[] = [
    { key: 'chat', label: '对话', icon: <MessageSquare size={13} /> },
    { key: 'sections', label: '分段', icon: <BookOpen size={13} /> },
    { key: 'settings', label: '设置', icon: <Settings size={13} /> },
  ];

  return (
    <div
      className={`fixed top-0 right-0 h-full w-[380px] bg-bg-secondary border-l border-border z-40 flex flex-col transition-transform duration-200 ${
        open ? 'translate-x-0' : 'translate-x-full'
      }`}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border shrink-0">
        <div className="flex gap-0.5">
          {tabs.map(t => (
            <button
              key={t.key}
              onClick={() => {
                setTab(t.key);
                if (t.key === 'sections' && !sections && !sectionsLoading) loadSections();
                if (t.key === 'settings' && providers.length === 0) loadSettings();
              }}
              className={`flex items-center gap-1 px-2 py-1.5 text-xs rounded transition ${
                tab === t.key ? 'bg-accent/20 text-accent font-medium' : 'text-muted hover:text-gray-300 hover:bg-bg-hover'
              }`}
            >
              {t.icon}{t.label}
            </button>
          ))}
        </div>
        <button onClick={onClose} className="p-1 text-muted hover:text-gray-200 transition">
          <X size={16} />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {tab === 'chat' && (
          <div className="flex flex-col flex-1 overflow-hidden">
            <div className="flex-1 overflow-y-auto p-3 space-y-3">
              {messages.length === 0 && (
                <div className="text-xs text-muted text-center py-8">
                  输入问题，AI 会基于文章内容回答<br />
                  也可以划词后选择"问AI"<br />
                  输入「搜 xxx」或点 🌐 联网搜索
                </div>
              )}
              {messages.map((m, i) => (
                m.role === 'system' ? (
                  <div key={i} className="text-[11px] leading-relaxed bg-bg-tertiary border border-border rounded-md p-2.5 text-gray-400">
                    <span className="flex items-center gap-1 text-[10px] text-accent mb-1"><Globe size={10} /> 联网结果</span>
                    <span className="whitespace-pre-wrap">{m.content.replace(/^联网搜索结果:\n\n/, '')}</span>
                  </div>
                ) : (
                  <div key={i} className={`text-xs leading-relaxed ${m.role === 'user' ? 'text-accent' : 'text-gray-300'}`}>
                    <span className="text-[10px] text-muted block mb-0.5">{m.role === 'user' ? '你' : 'AI'}</span>
                    <span className="whitespace-pre-wrap">{m.content}</span>
                  </div>
                )
              ))}
              {(sending || searching) && (
                <div className="flex items-center gap-1 text-xs text-muted">
                  <Loader2 size={12} className="animate-spin" /> {searching ? '搜索中...' : '思考中...'}
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>
            <div className="flex gap-2 p-2 border-t border-border shrink-0">
              <input
                value={input}
                onChange={e => setInput(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); } }}
                placeholder="问关于这篇文章的问题，或「搜 xxx」..."
                className="flex-1 px-2.5 py-1.5 bg-bg-tertiary border border-border rounded-md text-xs text-gray-200 placeholder:text-muted focus:outline-none focus:ring-1 focus:ring-accent"
                disabled={sending || searching}
              />
              <button
                onClick={handleSearchClick}
                disabled={sending || searching || !input.trim()}
                title="联网搜索当前输入"
                className="px-2.5 py-1.5 bg-bg-tertiary hover:bg-bg-hover text-gray-300 border border-border rounded-md disabled:opacity-40 transition"
              >
                <Globe size={12} />
              </button>
              <button
                onClick={handleSend}
                disabled={sending || searching || !input.trim()}
                className="px-2.5 py-1.5 bg-accent hover:bg-accent-hover text-white rounded-md disabled:opacity-40 transition"
              >
                <Send size={12} />
              </button>
            </div>
          </div>
        )}

        {tab === 'sections' && (
          <div className="flex-1 overflow-y-auto p-3">
            {sectionsLoading ? (
              <div className="flex items-center justify-center gap-2 py-8 text-xs text-muted">
                <Loader2 size={14} className="animate-spin" /> 分析文章结构...
              </div>
            ) : sections ? (
              <div className="space-y-1">
                {sections.overview && (
                  <div className="bg-bg-tertiary rounded-lg p-3 text-xs leading-relaxed text-gray-300 mb-3">
                    <span className="text-muted font-medium">概要：</span>{sections.overview}
                  </div>
                )}
                {sectionNotice && (
                  <div className="text-[11px] text-amber-400 px-2 py-1">{sectionNotice}</div>
                )}
                {/* 大纲式：左侧层级竖线 + 序号锚点，点击跳转原文 */}
                <div className="border-l-2 border-border/60 pl-1">
                  {sections.sections?.map(sec => (
                    <button
                      key={sec.index}
                      onClick={() => handleSectionJump(sec)}
                      title={sec.start_text ? '跳转到原文对应位置' : '该段无定位锚点'}
                      className="group w-full text-left flex items-start gap-2 py-2 pl-2 pr-2 -ml-px border-l-2 border-transparent hover:border-accent hover:bg-bg-hover rounded-r transition"
                    >
                      <span className="shrink-0 w-5 h-5 bg-accent/80 group-hover:bg-accent text-white text-[11px] rounded-full flex items-center justify-center font-semibold mt-0.5">
                        {sec.index}
                      </span>
                      <div className="min-w-0 flex-1">
                        <p className="text-xs text-gray-200 flex items-start gap-1">
                          <span className="flex-1">{sec.summary}</span>
                          <Anchor size={11} className="shrink-0 mt-0.5 text-muted opacity-0 group-hover:opacity-100 transition" />
                        </p>
                        {sec.keywords?.length > 0 && (
                          <div className="flex flex-wrap gap-1 mt-1">
                            {sec.keywords.map(k => (
                              <span key={k} className="bg-accent/10 text-accent text-[10px] px-1.5 py-0.5 rounded">{k}</span>
                            ))}
                          </div>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
                <button onClick={loadSections} className="w-full text-xs text-muted hover:text-accent py-2 mt-2 transition">
                  重新分析
                </button>
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center py-8 gap-2">
                <BookOpen size={24} className="text-muted" />
                <button onClick={loadSections} className="px-3 py-1.5 bg-accent hover:bg-accent-hover text-white text-xs rounded-md transition">
                  分析文章结构
                </button>
              </div>
            )}
          </div>
        )}

        {tab === 'settings' && (
          <div className="flex-1 overflow-y-auto p-3 space-y-4">
            {settingsLoading ? (
              <div className="flex items-center justify-center gap-2 py-8 text-xs text-muted">
                <Loader2 size={14} className="animate-spin" /> 加载设置...
              </div>
            ) : (
              <>
                {/* Provider */}
                <div>
                  <label className="text-xs text-muted block mb-1">AI 供应商</label>
                  <select
                    value={selectedProvider}
                    onChange={e => handleProviderChange(e.target.value)}
                    className="w-full px-2.5 py-1.5 bg-bg-tertiary border border-border rounded-md text-xs text-gray-200 focus:outline-none focus:ring-1 focus:ring-accent"
                  >
                    {providers.length === 0 && <option value="">（无可用供应商）</option>}
                    {providers.map(p => (
                      <option key={p.name} value={p.name}>{p.name}</option>
                    ))}
                  </select>
                </div>

                {/* Model */}
                <div>
                  <label className="text-xs text-muted block mb-1">模型</label>
                  <select
                    value={selectedModel}
                    onChange={e => setSelectedModel(e.target.value)}
                    className="w-full px-2.5 py-1.5 bg-bg-tertiary border border-border rounded-md text-xs text-gray-200 focus:outline-none focus:ring-1 focus:ring-accent"
                  >
                    {currentModels.length === 0 && <option value="">（无可用模型）</option>}
                    {currentModels.map(m => (
                      <option key={m} value={m}>{m}</option>
                    ))}
                  </select>
                </div>

                <p className="text-[11px] text-muted">
                  供应商与模型来自 weft Providers 配置
                </p>

                {/* Font size */}
                <div>
                  <label className="text-xs text-muted block mb-1">阅读字号: {fontSize}px</label>
                  <input
                    type="range"
                    min={12}
                    max={24}
                    value={fontSize}
                    onChange={e => onFontSizeChange(Number(e.target.value))}
                    className="w-full accent-accent"
                  />
                </div>

                {/* Translate settings */}
                <div className="border-t border-border pt-3">
                  <p className="text-xs text-gray-300 font-medium mb-2">翻译服务</p>

                  <div className="space-y-3">
                    <div>
                      <label className="text-xs text-muted block mb-1">翻译供应商</label>
                      <select
                        value={translateProvider}
                        onChange={e => setTranslateProvider(e.target.value)}
                        className="w-full px-2.5 py-1.5 bg-bg-tertiary border border-border rounded-md text-xs text-gray-200 focus:outline-none focus:ring-1 focus:ring-accent"
                      >
                        <option value="mymemory">MyMemory (免费)</option>
                        <option value="google">Google Translate</option>
                        <option value="microsoft">Microsoft Translator</option>
                        <option value="llm">LLM 翻译</option>
                      </select>
                    </div>

                    <div>
                      <label className="text-xs text-muted block mb-1">目标语言</label>
                      <select
                        value={translateTarget}
                        onChange={e => setTranslateTarget(e.target.value)}
                        className="w-full px-2.5 py-1.5 bg-bg-tertiary border border-border rounded-md text-xs text-gray-200 focus:outline-none focus:ring-1 focus:ring-accent"
                      >
                        <option value="zh">中文</option>
                        <option value="en">English</option>
                        <option value="ja">日本語</option>
                        <option value="ko">한국어</option>
                      </select>
                    </div>

                    {translateProvider === 'microsoft' && (
                      <>
                        <div>
                          <label className="text-xs text-muted block mb-1">API Key</label>
                          <input
                            type="password"
                            value={translateApiKey}
                            onChange={e => setTranslateApiKey(e.target.value)}
                            placeholder="Microsoft Translator API Key"
                            className="w-full px-2.5 py-1.5 bg-bg-tertiary border border-border rounded-md text-xs text-gray-200 placeholder:text-muted focus:outline-none focus:ring-1 focus:ring-accent"
                          />
                        </div>
                        <div>
                          <label className="text-xs text-muted block mb-1">Region (可选)</label>
                          <input
                            type="text"
                            value={translateRegion}
                            onChange={e => setTranslateRegion(e.target.value)}
                            placeholder="如: eastasia"
                            className="w-full px-2.5 py-1.5 bg-bg-tertiary border border-border rounded-md text-xs text-gray-200 placeholder:text-muted focus:outline-none focus:ring-1 focus:ring-accent"
                          />
                        </div>
                      </>
                    )}
                  </div>
                </div>

                {settingsMsg && <p className="text-xs text-accent">{settingsMsg}</p>}

                <button
                  onClick={handleSaveSettings}
                  disabled={saving || !selectedModel}
                  className="w-full px-3 py-2 bg-accent hover:bg-accent-hover text-white text-xs rounded-md font-medium disabled:opacity-40 transition"
                >
                  {saving ? '保存中...' : '保存设置'}
                </button>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
