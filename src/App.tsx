import { useEffect, useRef, useState } from "react";
import { api, events } from "./api";
import type {
  Config,
  MonitorSnapshot,
  OllamaInfo,
  RecommendResult,
} from "./types";
import StatusTab from "./components/StatusTab";
import SettingsTab from "./components/SettingsTab";
import ModelsTab from "./components/ModelsTab";

type Tab = "status" | "settings" | "models";

export default function App() {
  const [tab, setTab] = useState<Tab>("status");
  const [config, setConfig] = useState<Config | null>(null);
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [gpu, setGpu] = useState<RecommendResult | null>(null);
  const [ollama, setOllama] = useState<OllamaInfo | null>(null);
  const [pullLog, setPullLog] = useState<string[]>([]);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const showToast = (msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 4500);
  };

  const refreshOllama = async (url?: string) => {
    const u = url ?? config?.ollamaUrl ?? "http://127.0.0.1:11434";
    const info = await api.ollamaInfo(u).catch(() => null);
    setOllama(info);
    return info;
  };

  useEffect(() => {
    let disposed = false;
    (async () => {
      try {
        const cfg = await api.getConfig();
        if (disposed) return;
        setConfig(cfg);
        setSnapshot(await api.getMonitorState());
      } catch (e) {
        showToast(`初始化失败：${e}`);
      }
      setGpu(await api.getRecommendation().catch(() => null));
      refreshOllama();
    })();
    const unsubs = [
      events.onTick((t) =>
        setSnapshot((s) =>
          s ? { ...s, lastTicks: [...s.lastTicks.slice(-99), t] } : s
        )
      ),
      events.onState((s) =>
        setSnapshot((prev) => (prev ? { ...prev, running: s.running } : prev))
      ),
      events.onReminder((r) => showToast(`【${r.title}】${r.text}`)),
      events.onPull((p) => setPullLog((l) => [...l.slice(-99), p.line])),
    ];
    return () => {
      disposed = true;
      unsubs.forEach((u) => u.then((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const startMonitoring = async (cfg: Config) => {
    try {
      await api.startMonitoring(cfg);
      setConfig(cfg);
      setSnapshot(await api.getMonitorState());
      showToast("监控已启动，开始检测专注度…");
    } catch (e) {
      showToast(`启动失败：${e}`);
    }
  };

  const stopMonitoring = async () => {
    await api.stopMonitoring();
    setSnapshot(await api.getMonitorState());
    showToast("监控已停止");
  };

  const saveConfig = async (cfg: Config) => {
    await api.saveConfig(cfg);
    setConfig(cfg);
    showToast("配置已保存");
  };

  return (
    <div className="app">
      <header className="app-header">
        <div className="brand">
          <span className="logo">◎</span>
          <h1>FocusGuard</h1>
          <span className="subtitle">专注监控 · 原型</span>
        </div>
        <nav className="tabs">
          <button
            className={tab === "status" ? "tab active" : "tab"}
            onClick={() => setTab("status")}
          >
            监控状态
          </button>
          <button
            className={tab === "settings" ? "tab active" : "tab"}
            onClick={() => setTab("settings")}
          >
            设置
          </button>
          <button
            className={tab === "models" ? "tab active" : "tab"}
            onClick={() => setTab("models")}
          >
            模型
          </button>
        </nav>
        <div className="header-right">
          {snapshot?.running && <span className="badge running">● 监控中</span>}
        </div>
      </header>
      <main className="app-main">
        {tab === "status" && (
          <StatusTab
            config={config}
            snapshot={snapshot}
            onStart={startMonitoring}
            onStop={stopMonitoring}
          />
        )}
        {tab === "settings" && (
          <SettingsTab
            config={config}
            gpu={gpu}
            onSave={saveConfig}
            onToast={showToast}
          />
        )}
        {tab === "models" && (
          <ModelsTab
            ollama={ollama}
            refreshOllama={refreshOllama}
            pullLog={pullLog}
            onToast={showToast}
          />
        )}
      </main>
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
