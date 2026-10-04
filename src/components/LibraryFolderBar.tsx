import { useCallback, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';

interface Props {
  folders: string[];
  missing: string[];
  potplayerPath: string | null;
  onBrowse: (paths: string[]) => void;
  onRemove: (path: string) => void;
  onRescan: () => void;
  onSetPotPlayer: () => void;
}

function basename(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? p;
}

export default function LibraryFolderBar({
  folders, missing, potplayerPath,
  onBrowse, onRemove, onRescan, onSetPotPlayer,
}: Props) {
  const [manual, setManual] = useState(false);
  const [draft, setDraft] = useState('');

  const browse = useCallback(async () => {
    const picked = await open({ directory: true, multiple: true });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    if (paths.length > 0) onBrowse(paths);
  }, [onBrowse]);

  const submitManual = useCallback(() => {
    const p = draft.trim();
    if (!p) return;
    onBrowse([p]);
    setDraft('');
    setManual(false);
  }, [draft, onBrowse]);

  const closeManual = useCallback(() => {
    setManual(false);
    setDraft('');
  }, []);

  return (
    <div className="library-bar">
      <div className="library-folders">
        {folders.length === 0 && (
          <span className="library-hint">未添加目录 —— 用「浏览」、手动输入，或直接把文件夹拖进窗口</span>
        )}
        {folders.map((f) => {
          const bad = missing.includes(f);
          return (
            <span key={f} className={`folder-chip ${bad ? 'missing' : ''}`} title={bad ? '目录不可访问' : f}>
              {bad ? '⚠️' : '📁'} {basename(f)}
              {bad && <em>（不可访问）</em>}
              <button className="folder-chip-x" onClick={() => onRemove(f)} title="移除">✕</button>
            </span>
          );
        })}
      </div>

      <div style={{ flex: 1 }} />

      {manual && (
        <input
          className="search-input library-manual"
          placeholder="粘贴目录绝对路径，回车确认"
          autoFocus
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') submitManual();
            if (e.key === 'Escape') closeManual();
          }}
        />
      )}

      <button className="btn btn-secondary btn-sm" onClick={browse}>浏览…</button>
      <button className="btn btn-secondary btn-sm" onClick={() => (manual ? closeManual() : setManual(true))}>
        手动输入
      </button>
      <button className="btn btn-secondary btn-sm" onClick={onRescan}>重新扫描</button>
      <button
        className="btn btn-secondary btn-sm"
        onClick={onSetPotPlayer}
        title={potplayerPath ?? '未找到 PotPlayer，点击手动指定可执行文件'}
      >
        {potplayerPath ? '播放器 ✓' : '设置播放器'}
      </button>
    </div>
  );
}