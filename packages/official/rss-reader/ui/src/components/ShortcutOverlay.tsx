import { X } from 'lucide-react';

interface Props {
  open: boolean;
  onClose: () => void;
}

const shortcuts = [
  { key: 'Esc', desc: '关闭浮层 (快捷键→AI面板→退出禅模式)' },
  { key: 'F', desc: '进入/退出全屏禅模式' },
  { key: 'T', desc: '沉浸式翻译' },
  { key: 'A', desc: '打开/关闭 AI 面板' },
];

export function ShortcutOverlay({ open, onClose }: Props) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60" onClick={onClose}>
      <div
        className="bg-bg-secondary border border-border rounded-xl p-6 w-80 shadow-2xl"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-base font-semibold text-gray-100">快捷键</h3>
          <button onClick={onClose} className="text-muted hover:text-gray-200 transition">
            <X size={16} />
          </button>
        </div>
        <div className="space-y-3">
          {shortcuts.map(s => (
            <div key={s.key} className="flex items-center justify-between">
              <span className="text-sm text-gray-300">{s.desc}</span>
              <kbd className="px-2 py-0.5 bg-bg-primary border border-border rounded text-xs text-gray-400 font-mono">
                {s.key}
              </kbd>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
