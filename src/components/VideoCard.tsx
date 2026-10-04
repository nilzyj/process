import { useCallback, useRef, useState } from 'react';
import type { VideoFile } from '../types';
import { formatBytes } from '../types';

interface Props {
  video: VideoFile;
  onPlay: (path: string) => void;
  onReveal: (path: string) => void;
  onCopyPath: (path: string) => void;
}

export default function VideoCard({ video, onPlay, onReveal, onCopyPath }: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [playing, setPlaying] = useState(false);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const ext = video.path.split('.').pop()?.toUpperCase() ?? '';
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
      className="video-card"
      onClick={handleClick}
      onContextMenu={(e) => {
        e.preventDefault();
        openMenu();
      }}
      onMouseLeave={closeMenu}
      title={video.path}
    >
      <div className="video-thumb">
        <span className="video-ext">{ext}</span>
        {playing && <span className="video-playing">▶</span>}
      </div>

      <div className="video-name">{video.name || '--'}</div>

      <div className="video-meta">
        <span>{formatBytes(video.size)}</span>
        <span>{video.mtime ? new Date(video.mtime * 1000).toISOString().slice(0, 10) : '--'}</span>
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