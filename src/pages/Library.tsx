import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { open } from '@tauri-apps/plugin-dialog';
import type { LibraryConfig, ScanResult, VideoFile } from '../types';
import LibraryFolderBar from '../components/LibraryFolderBar';
import VideoCard from '../components/VideoCard';

export default function Library() {
  const [videos, setVideos] = useState<VideoFile[]>([]);
  const [folders, setFolders] = useState<string[]>([]);
  const [missing, setMissing] = useState<string[]>([]);
  const [potplayer, setPotplayer] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState('');
  const [note, setNote] = useState('');
  const [dragOver, setDragOver] = useState(false);

  const applyResult = useCallback((r: ScanResult) => {
    setVideos(r.videos);
    setMissing(r.missing);
    setScanning(false);
    setNote(
      r.skipped > 0 ? `已跳过 ${r.skipped} 个无法读取的文件` : '',
    );
  }, []);

  const rescan = useCallback(async () => {
    setScanning(true);
    setError('');
    try {
      applyResult(await invoke<ScanResult>('rescan_library'));
    } catch (e) {
      setError(String(e));
      setScanning(false);
    }
  }, [applyResult]);

  const addFolders = useCallback(async (paths: string[]) => {
    setScanning(true);
    setError('');
    try {
      applyResult(await invoke<ScanResult>('add_library_folder', { paths }));
    } catch (e) {
      setError(String(e));
      setScanning(false);
    }
  }, [applyResult]);

  const removeFolder = useCallback(async (path: string) => {
    try {
      applyResult(await invoke<ScanResult>('remove_library_folder', { path }));
    } catch (e) {
      setError(String(e));
    }
  }, [applyResult]);

  // 启动时加载配置；有目录则自动扫描
  useEffect(() => {
    const init = async () => {
      try {
        const cfg = await invoke<LibraryConfig>('get_library_config');
        setFolders(cfg.folders);
        setPotplayer(cfg.potplayer_path);

        // 未手动指定时，探测自动定位是否可用，好让用户看到真实状态
        if (!cfg.potplayer_path) {
          invoke<string | null>('locate_potplayer')
            .then((found) => { if (found) setPotplayer(found); })
            .catch(() => {});
        }

        if (cfg.folders.length > 0) {
          applyResult(await invoke<ScanResult>('rescan_library'));
        } else {
          setScanning(false);
        }
      } catch (e) {
        setError(String(e));
        setScanning(false);
      }
    };
    init();
  }, [applyResult]);

  // 拖放：Tauri 原生事件，core:default 已含 allow-listen，无需额外权限
  useEffect(() => {
    let dispose: (() => void) | undefined;
    let cancelled = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === 'over') setDragOver(true);
        else if (p.type === 'leave') setDragOver(false);
        else if (p.type === 'drop') {
          setDragOver(false);
          if (p.paths.length > 0) addFolders(p.paths);
        }
      })
      .then((un) => {
        if (cancelled) un();
        else dispose = un;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      dispose?.();
    };
  }, [addFolders]);

  const handlePlay = useCallback(async (path: string) => {
    setError('');
    try {
      await invoke<string>('play_video', { path });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleReveal = useCallback(async (path: string) => {
    try {
      await invoke<string>('reveal_in_explorer', { path });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleCopyPath = useCallback(async (path: string) => {
    try {
      await navigator.clipboard.writeText(path);
      setNote('路径已复制');
      setTimeout(() => setNote(''), 1500);
    } catch {
      setError('复制失败，请手动复制');
    }
  }, []);

  const handleSetPotPlayer = useCallback(async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      title: '选择 PotPlayer 可执行文件',
    });
    const exe = Array.isArray(picked) ? picked[0] : picked;
    if (!exe) return;
    const cfg = await invoke<LibraryConfig>('get_library_config');
    cfg.potplayer_path = exe;
    await invoke<string>('save_library_config', { cfg });
    setPotplayer(exe);
    setNote('播放器路径已保存');
    setTimeout(() => setNote(''), 2000);
  }, []);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return videos;
    return videos.filter(
      (v) => v.name.toLowerCase().includes(q) || v.path.toLowerCase().includes(q),
    );
  }, [videos, search]);

  return (
    <>
      <LibraryFolderBar
        folders={folders}
        missing={missing}
        potplayerPath={potplayer}
        onBrowse={addFolders}
        onRemove={removeFolder}
        onRescan={rescan}
        onSetPotPlayer={handleSetPotPlayer}
      />

      <div className="list-info">
        <span>
          {search ? `${filtered.length} / ${videos.length} 个文件` : `共 ${videos.length} 个文件`}
        </span>
        {note && <span style={{ color: 'var(--info)', fontSize: 12 }}>{note}</span>}
      </div>

      <div className="library-filter">
        <div className="search-wrapper">
          <span className="search-icon">🔍</span>
          <input
            className="search-input"
            placeholder="搜索文件名或路径..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>

      {scanning ? (
        <div className="empty-state">
          <div className="loading-spinner" />
          <p>正在扫描目录...</p>
        </div>
      ) : error ? (
        <div className="empty-state">
          <div className="icon">⚠️</div>
          <p style={{ color: 'var(--danger)', wordBreak: 'break-all', maxWidth: '80%' }}>{error}</p>
        </div>
      ) : videos.length === 0 ? (
        <div className="empty-state">
          <div className="icon">📼</div>
          <p>{folders.length === 0 ? '还没有添加媒体库目录' : '目录中没有找到视频文件'}</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="empty-state">
          <div className="icon">🔍</div>
          <p>没有匹配的文件</p>
        </div>
      ) : (
        <div className="video-grid">
          {filtered.map((v) => (
            <VideoCard
              key={v.path}
              video={v}
              onPlay={handlePlay}
              onReveal={handleReveal}
              onCopyPath={handleCopyPath}
            />
          ))}
        </div>
      )}

      {dragOver && (
        <div className="drop-overlay">
          <div className="drop-hint">松开以添加为媒体库目录</div>
        </div>
      )}
    </>
  );
}