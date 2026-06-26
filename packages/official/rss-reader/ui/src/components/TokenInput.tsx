import { useState } from 'react';
import { KeyRound } from 'lucide-react';
import { setToken } from '../api';

interface Props {
  onTokenSet: () => void;
}

export function TokenInput({ onTokenSet }: Props) {
  const [value, setValue] = useState('');

  const handleSubmit = () => {
    if (value.trim()) {
      setToken(value.trim());
      onTokenSet();
    }
  };

  return (
    <div className="flex items-center justify-center h-full">
      <div className="bg-bg-secondary border border-border rounded-xl p-8 w-96 shadow-xl text-center">
        <KeyRound size={32} className="mx-auto text-accent mb-4" />
        <h2 className="text-lg font-semibold text-gray-100 mb-2">连接 Weft Core</h2>
        <p className="text-xs text-muted mb-4">
          需要 loopback token 访问 API。<br />
          从 <code className="bg-bg-primary px-1 py-0.5 rounded text-xs">data/runtime-token</code> 文件复制粘贴。
        </p>
        <input
          type="text"
          value={value}
          onChange={e => setValue(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleSubmit()}
          placeholder="粘贴 runtime-token..."
          className="w-full px-3 py-2 bg-bg-primary border border-border rounded-md text-sm text-gray-200 placeholder:text-muted focus:outline-none focus:ring-1 focus:ring-accent mb-4"
          autoFocus
        />
        <button
          onClick={handleSubmit}
          disabled={!value.trim()}
          className="w-full px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-md font-medium text-sm disabled:opacity-40 transition"
        >
          连接
        </button>
      </div>
    </div>
  );
}
