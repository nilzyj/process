import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open, confirm, message } from '@tauri-apps/plugin-dialog';
import type { AppConfig, SnapshotInfo, StorageInfo } from '../types';
import { formatBytes } from '../types';

export default function Settings() {
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [cfg, setCfg] = useState<AppConfig | null>(null);
  const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([]);
  const [legacyCreds, setLegacyCreds] = useState(false);
  const [busy, setBusy] = useState('');
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  const reload = useCallback(async () => {
    const [i, c, s, legacy] = await Promise.all([
      invoke<StorageInfo>('get_storage_info'),
      invoke<AppConfig>('get_app_config'),
      invoke<SnapshotInfo[]>('list_snapshots'),
      invoke<boolean>('legacy_credentials_present'),
    ]);
    setInfo(i);
    setCfg(c);
    setSnapshots(s);
    setLegacyCreds(legacy);
  }, []);

  useEffect(() => {
    reload().catch((e) => setMsg({ text: String(e), ok: false }));
  }, [reload]);

  const flash = useCallback((text: string, ok = true) => {
    setMsg({ text, ok });
    setTimeout(() => setMsg(null), 4000);
  }, []);

  const persist = useCallback(
    async (next: AppConfig) => {
      await invoke<string>('save_app_config', { cfg: next });
      setCfg(next);
    },
    [],
  );

  const pickDataDir = useCallback(async () => {
    if (!cfg) return;
    const dir = await open({ directory: true, multiple: false, title: '选择数据库存放目录' });
    if (typeof dir !== 'string') return;
    if (
      !(await confirm(
        '更改数据目录只影响新数据库文件的位置，不会移动现有数据。\n\n' +
          '原数据库文件会留在原处。若要继续使用原数据，请把新目录改回原路径。\n\n仍要更改吗？',
        { title: '更改数据目录', kind: 'warning' },
      ))
    ) {
      return;
    }
    await persist({ ...cfg, data_dir: dir });
    flash('已保存，重启程序后生效');
  }, [cfg, persist, flash]);

  const pickSnapshotDir = useCallback(async () => {
    if (!cfg) return;
    const dir = await open({ directory: true, multiple: false, title: '选择坚果云同步文件夹' });
    if (typeof dir !== 'string') return;
    await persist({ ...cfg, snapshot_dir: dir });
    await reload();
    flash('快照目录已更新');
  }, [cfg, persist, reload, flash]);

  const doExport = useCallback(async () => {
    if (!cfg?.snapshot_dir) {
      flash('请先设置快照目录', false);
      return;
    }
    setBusy('export');
    try {
      const path = await invoke<string>('export_snapshot');
      await reload();
      flash(`已导出到 ${path}`);
    } catch (e) {
      flash(String(e), false);
    } finally {
      setBusy('');
    }
  }, [cfg, reload, flash]);

  const doRestore = useCallback(async (path: string, name: string) => {
    if (
      !(await confirm(
        `将用快照「${name}」覆盖当前全部记录。\n\n` +
          '当前数据会先自动导出一份快照，可从下方列表回退。确定继续？',
        { title: '从快照恢复', kind: 'warning' },
      ))
    ) {
      return;
    }
    setBusy(path);
    try {
      const n = await invoke<number>('restore_snapshot', { path });
      await reload();
      flash(`已恢复 ${n} 条记录`);
    } catch (e) {
      flash(String(e), false);
    } finally {
      setBusy('');
    }
  }, [reload, flash]);

  const doImport = useCallback(async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      title: '选择导出的 JSON 文件',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    });
    if (typeof picked !== 'string') return;
    if (
      !(await confirm('导入会覆盖当前全部记录（当前数据会先自动备份为快照）。继续？', {
        title: '导入记录',
        kind: 'warning',
      }))
    ) {
      return;
    }
    setBusy('import');
    try {
      const n = await invoke<number>('import_records', { path: picked });
      await reload();
      flash(`已导入 ${n} 条记录`);
    } catch (e) {
      flash(String(e), false);
    } finally {
      setBusy('');
    }
  }, [reload, flash]);

  if (!cfg || !info) {
    return (
      <div className="empty-state">
        <div className="loading-spinner" />
        <p>加载中...</p>
      </div>
    );
  }

  return (
    <div className="settings-page">
      {legacyCreds && (
        <div className="settings-warn">
          <strong>检测到旧版 MySQL 配置</strong>
          <p>
            <code>~/.process-app/config.json</code> 仍是旧的扁平结构，其中的数据库密码已不再被程序读取，
            但仍以明文留在磁盘上。建议手动删除该文件。
          </p>
        </div>
      )}

      {msg && (
        <div className="settings-msg" style={{ color: msg.ok ? 'var(--success)' : 'var(--danger)' }}>
          {msg.text}
        </div>
      )}

      <section className="settings-section">
        <h3>本地数据库</h3>
        <div className="settings-row">
          <span className="settings-label">数据库文件</span>
          <code className="settings-value">{info.database_path}</code>
          <button className="btn btn-secondary btn-sm" onClick={pickDataDir}>
            更改目录
          </button>
        </div>
        <div className="settings-row">
          <span className="settings-label">记录数</span>
          <span>{info.record_count}</span>
        </div>
        <p className="settings-hint">
          SQLite 单文件存储。刻意不放在坚果云同步目录 —— 它是 .db + -wal + -shm 多文件，
          WAL 频繁变动会导致同步出损坏状态。备份请用下方快照。
        </p>
      </section>

      <section className="settings-section">
        <h3>快照备份</h3>
        <div className="settings-row">
          <span className="settings-label">快照目录</span>
          <code className="settings-value">{cfg.snapshot_dir ?? '未设置'}</code>
          <button className="btn btn-secondary btn-sm" onClick={pickSnapshotDir}>
            选择文件夹
          </button>
        </div>
        <div className="settings-row">
          <span className="settings-label">保留份数</span>
          <input
            className="settings-num"
            type="number"
            min={1}
            max={100}
            value={cfg.keep_snapshots}
            onChange={(e) => setCfg({ ...cfg, keep_snapshots: Math.max(1, Number(e.target.value) || 1) })}
            onBlur={() => persist(cfg)}
          />
          <button
            className="btn btn-secondary btn-sm"
            onClick={() => pickSnapshotDir()}
            title="设为坚果云同步文件夹后，导出的快照会被自动同步上云"
          >
            说明
          </button>
        </div>
        <div className="settings-row">
          <button className="btn btn-primary btn-sm" onClick={doExport} disabled={busy === 'export'}>
            {busy === 'export' ? '导出中...' : '立即导出快照'}
          </button>
        </div>
      </section>

      <section className="settings-section">
        <h3>快照列表（{snapshots.length}）</h3>
        {snapshots.length === 0 ? (
          <p className="settings-hint">暂无快照</p>
        ) : (
          <div className="settings-snapshots">
            {snapshots.map((s) => (
              <div key={s.path} className="settings-snapshot">
                <span className="settings-snapshot-name">{s.name}</span>
                <span className="settings-snapshot-meta">
                  {formatBytes(s.size_bytes)}
                  {s.modified ? ` · ${new Date(s.modified * 1000).toLocaleString('zh-CN')}` : ''}
                </span>
                <button
                  className="btn btn-secondary btn-sm"
                  onClick={() => doRestore(s.path, s.name)}
                  disabled={busy === s.path}
                >
                  {busy === s.path ? '恢复中...' : '恢复'}
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="settings-section">
        <h3>导入记录</h3>
        <p className="settings-hint">
          从导出的 JSON 文件导入，用于从 MySQL 迁移或手工恢复。文件可为快照格式
          （含 records 与 checksum）或纯记录数组。
        </p>
        <div className="settings-row">
          <button className="btn btn-secondary btn-sm" onClick={doImport} disabled={busy === 'import'}>
            {busy === 'import' ? '导入中...' : '选择 JSON 文件导入'}
          </button>
          <button
            className="btn btn-secondary btn-sm"
            onClick={async () => {
              await message(
                '将 ~/.process-app/config.json 改名为 config.json.bak 即可让程序忽略它。',
                { title: '如何清理旧凭据' },
              );
            }}
          >
            如何清理旧凭据
          </button>
        </div>
      </section>
    </div>
  );
}