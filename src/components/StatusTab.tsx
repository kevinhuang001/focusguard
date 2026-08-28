import { useEffect, useState } from "react";
import { api } from "../api";
import type { Config, MonitorSnapshot, MonitorTick } from "../types";
import { validateConfig } from "../validate";

function fmtDuration(totalSec: number): string {
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function fmtClock(ts: number): string {
  return new Date(ts).toLocaleTimeString("zh-CN", { hour12: false });
}

function SourceCard({ title, tick }: { title: string; tick?: MonitorTick }) {
  if (!tick) {
    return (
      <div className="source-card">
        <h3>{title}</h3>
        <p className="hint">等待检测…</p>
      </div>
    );
  }
  if (tick.error) {
    return (
      <div className="source-card error">
        <h3>{title}</h3>
        <p className="err">⚠ {tick.error}</p>
        <p className="hint">{fmtClock(tick.ts)}</p>
      </div>
    );
  }
  return (
    <div className={`source-card ${tick.focused ? "ok" : "warn"}`}>
      <h3>{title}</h3>
      <p className="verdict">
        <span className="dot" />
        {tick.focused ? "专注" : "疑似开小差"}
      </p>
      <p className="reason">{tick.reason || "（无原因）"}</p>
      <p className="hint">
        {fmtClock(tick.ts)} · {tick.model} · {tick.durationMs} ms
      </p>
    </div>
  );
}

interface Props {
  config: Config | null;
  snapshot: MonitorSnapshot | null;
  onStart: (cfg: Config) => Promise<void>;
  onStop: () => Promise<void>;
  onGoSettings: () => void;
}

export default function StatusTab({
  config,
  snapshot,
  onStart,
  onStop,
  onGoSettings,
}: Props) {
  const [now, setNow] = useState(Date.now());
  const [preview, setPreview] = useState<string | null>(null);
  useEffect(() => {
    const t = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(t);
  }, []);

  const openPreview = async (path?: string) => {
    if (!path) return;
    try {
      setPreview(await api.readHistoryImage(path));
    } catch {
      /* 忽略：图片可能已被清理 */
    }
  };

  if (!config || !snapshot) {
    return <div className="placeholder">加载中…</div>;
  }

  const ticks = snapshot.lastTicks;
  const latest = ticks.length ? ticks[ticks.length - 1] : undefined;
  const screenTick = [...ticks].reverse().find((t) => t.source === "screen");
  const cameraTick = [...ticks].reverse().find((t) => t.source === "camera");

  let status: "idle" | "pending" | "focused" | "distracted" = "idle";
  if (snapshot.running) {
    if (latest?.error) status = "pending";
    else if (latest?.focused) status = "focused";
    else if (latest) status = "distracted";
    else status = "pending";
  }
  const statusText: Record<string, string> = {
    idle: "未运行",
    pending: "检测中…",
    focused: "专注中",
    distracted: "疑似开小差",
  };
  const elapsed = snapshot.startedAt
    ? Math.floor((now - snapshot.startedAt) / 1000)
    : 0;
  const history = [...ticks].slice(-30).reverse();

  // 配置未完成时禁止启动
  const validation = config ? validateConfig(config) : { ok: false, errors: [] };
  const canStart = !snapshot.running && validation.ok;

  return (
    <div className="status">
      <section className={`status-card ${status}`}>
        <div className="status-main">
          <span className="status-dot" />
          <div>
            <p className="status-text">{statusText[status]}</p>
            {snapshot.running && (
              <p className="status-sub">
                已持续 {fmtDuration(elapsed)} · 连续开小差 {snapshot.missCount} 次
                {snapshot.lastReminderAt
                  ? ` · 上次提醒 ${fmtClock(snapshot.lastReminderAt)}`
                  : ""}
              </p>
            )}
            {!snapshot.running && !validation.ok && (
              <p className="status-sub warn-text">
                配置未完成：{validation.errors[0]}（
                <a onClick={onGoSettings} className="link-btn">
                  去设置
                </a>
                ）
              </p>
            )}
          </div>
        </div>
        {snapshot.running ? (
          <button className="btn danger" onClick={onStop}>
            停止监控
          </button>
        ) : (
          <button
            className="btn primary big"
            disabled={!canStart}
            onClick={() => onStart(config)}
          >
            {validation.ok ? "开始监控" : "请先完成设置"}
          </button>
        )}
      </section>

      <section className="sources">
        <SourceCard title="📺 屏幕" tick={screenTick} />
        <SourceCard title="📷 摄像头" tick={cameraTick} />
      </section>

      <section className="card">
        <h2>检测历史</h2>
        {history.length === 0 ? (
          <p className="hint">暂无记录。开始监控后，这里会显示每次检测结果。</p>
        ) : (
          <ul className="history">
            {history.map((t, i) => (
              <li key={`${t.ts}-${i}`} className={t.error ? "err" : t.focused ? "ok" : "warn"}>
                <span className="h-time">{fmtClock(t.ts)}</span>
                <span className="h-source">
                  {t.source === "screen" ? "屏幕" : "摄像头"}
                </span>
                <span className="h-verdict">
                  {t.error ? "出错" : t.focused ? "专注" : "开小差"}
                </span>
                <span className="h-reason">
                  {t.error ?? t.reason ?? ""}
                </span>
                {t.imagePath && (
                  <button className="view-btn" onClick={() => openPreview(t.imagePath)}>
                    查看
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {preview && (
        <div className="preview-overlay" onClick={() => setPreview(null)}>
          <img src={preview} alt="历史检测画面" className="preview-img" />
          <div className="preview-close">点击任意处关闭</div>
        </div>
      )}
    </div>
  );
}
