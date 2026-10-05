import { memo, useCallback } from 'react';
import type { VideoFile } from '../types';
import VideoCard from './VideoCard';

interface Props {
  name: string;
  dir: string;
  items: VideoFile[];
  collapsed: boolean;
  onToggle: (dir: string) => void;
  onPlay: (path: string) => void;
  onReveal: (path: string) => void;
  onCopyPath: (path: string) => void;
}

/**
 * 单独抽出并 memo 是性能关键。
 * 分组可达上千个，若在列表里内联渲染，折叠任一组都会让其余所有组的
 * 数千个 VideoCard 一起重渲染。memo 后只有被切换的那组会更新。
 */
function VideoGroupBase({
  name, dir, items, collapsed, onToggle, onPlay, onReveal, onCopyPath,
}: Props) {
  const handleToggle = useCallback(() => onToggle(dir), [onToggle, dir]);

  return (
    <section className="video-group">
      <header
        className={`video-group-head ${collapsed ? 'is-collapsed' : ''}`}
        onClick={handleToggle}
        title={dir}
      >
        <span className="video-group-caret">{collapsed ? '▸' : '▾'}</span>
        <span className="video-group-name">{name}</span>
        <span className="video-group-count">{items.length}</span>
      </header>

      {!collapsed && (
        <div className="video-grid">
          {items.map((v) => (
            <VideoCard
              key={v.path}
              video={v}
              onPlay={onPlay}
              onReveal={onReveal}
              onCopyPath={onCopyPath}
            />
          ))}
        </div>
      )}
    </section>
  );
}

export default memo(VideoGroupBase);