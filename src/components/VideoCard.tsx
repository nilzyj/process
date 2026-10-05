import { memo, useCallback, useRef, useState } from 'react';
import type { VideoFile } from '../types';
import { formatBytes } from '../types';

interface Props {
  video: VideoFile;
  onPlay: (path: string) => void;
  onReveal: (path: string) => void;
  onCopyPath: (path: string) => void;
}

/** 常见容器给个稳定配色，扫一眼就能区分格式；其余走灰色 */
const EXT_COLOR: Record<string, string> = {
  MKV: '#5eead4',
  MP4: '#60a5fa',
  AVI: '#c084fc',
  MOV: '#f472b6',
  TS: '#fbbf24',
  M2TS: '#fb923c',
  WEBM: '#34d399',
  WMV: '#a3e635',
  RMVB: '#f87171',
  RM: '#f87171',
  FLV: '#22d3ee',
  VOB: '#d8b4fe',
  MPG: '#f0abfc',
  MPEG: '#f0abfc',
};

function formatDate(unixSec: number): string {
  if (!unixSec) return '';
  const d = new Date(unixSec * 1000);
  if (Number.isNaN(d.getTime())) return '';
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
    d.getDate(),
  ).padStart(2, '0')}`;
}

function VideoCardBase({ video, onPlay, onReveal, onCopyPath }: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [playing, setPlaying] = useState(false);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const ext = video.path.split('.').pop()?.toUpperCase() ?? '';
  const extColor = EXT_COLOR[ext];
  const folder = video.path.slice(0, Math.max(video.path.lastIndexOf('\\'), video.path.lastIndexOf('/')));

  const closeMenu = useCallback(() => {
    if (hideTimer.current) {
      clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
    setMenuOpen(false);
  }, []);

  const openMenu = useCallback(() => {
    if (hideTimer.current) {
      clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
    setMenuOpen(true);
  }, []);

  const handleClick = useCallback(() => {
    setPlaying(true);
    onPlay(video.path);
    setTimeout(() => setPlaying(false), 600);
  }, [video.path, onPlay]);

  return (
    <div
      className={`video-card ${playing ? 'is-playing' : ''}`}
      onClick={handleClick}
      onContextMenu={(e) => {
        e.preventDefault();
        openMenu();
      }}
      onMouseLeave={closeMenu}
      title={video.path}
    >
      <div className="video-name">{video.name || '--'}</div>

      <div className="video-meta">
        <span
          className="video-ext"
          style={extColor ? { color: extColor, borderColor: `${extColor}44`, background: `${extColor}14` } : undefined}
        >
          {ext}
        </span>
        <span className="video-size">{formatBytes(video.size)}</span>
        {formatDate(video.mtime) && <span className="video-date">{formatDate(video.mtime)}</span>}
        {playing && <span className="video-playing">▶</span>}
      </div>

      {menuOpen && (
        <div className="video-context" onClick={(e) => e.stopPropagation()}>
          <button
            className="video-context-item"
            onClick={() => {
              closeMenu();
              onPlay(video.path);
            }}
          >
            ▶ 播放
          </button>
          <button
            className="video-context-item"
            onClick={() => {
              closeMenu();
              onReveal(video.path);
            }}
          >
            📂 打开所在文件夹
          </button>
          <button
            className="video-context-item"
            onClick={() => {
              closeMenu();
              onCopyPath(video.path);
            }}
          >
            📋 复制完整路径
          </button>
          <div className="video-context-folder" title={folder}>
            {folder}
          </div>
        </div>
      )}
    </div>
  );
}

// 库里可达上万文件，任何父组件重渲染都会波及全部卡片。
// props 中的 video 与回调均为稳定引用，memo 可完全跳过更新。
export default memo(VideoCardBase);