import { useCallback, useEffect, useRef, useState } from "react";
import { api, events } from "./api";
import type { Config, MonitorSnapshot, RecommendResult } from "./types";
import StatusTab from "./components/StatusTab";
import SettingsTab from "./components/SettingsTab";

type Tab = "status" | "settings";

export default function App() {
  const [tab, setTab] = useState<Tab>("status");
  const [config, setConfig] = useState<Config | null>(null);
  const [snapshot, setSnapshot] = useState<MonitorSnapshot | null>(null);
  const [gpu, setGpu] = useState<RecommendResult | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);
  const recApplied = useRef(false);
  const [isFirstRun, setIsFirstRun] = useState(false);

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 4500);
  }, []);

  useEffect(() => {
    let disposed = false;
    (async () => {
      try {
        const cfg = await api.getConfig();
        if (disposed) return;
        setConfig(cfg);
        setSnapshot(await api.getMonitorState());
        // 首次启动：直接进入配置引导
        if (!cfg.configured) {
          setIsFirstRun(true);
          setTab("settings");
        }
      } catch (e) {
        showToast(`初始化失败：${e}`);
      }
      setGpu(await api.getRecommendation().catch(() => null));
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
    ];
    return () => {
      disposed = true;
      unsubs.forEach((u) => u.then((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // GPU 推荐参数自动应用（只应用一次，之后以用户手动修改为准）
  useEffect(() => {
    if (gpu && config && !recApplied.current) {
      recApplied.current = true;
      setConfig((c) =>
        c
          ? {
              ...c,
              modelApi: { ...c.modelApi, model: gpu.model },
              intervalSecs: gpu.intervalSecs,
            }
          : c
      );
    }
  }, [gpu, config]);

  const startMonitoring = async (cfg: Config) => {
    try {
      await api.startMonitoring(cfg);
      setConfig(cfg);
      setIsFirstRun(false);
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
    const saved = { ...cfg, configured: true };
    setConfig(saved);
    setIsFirstRun(false);
    showToast("配置已保存");
  };

  const goSettings = () => setTab("settings");

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
        </nav>
        <div className="header-right">
          {snapshot?.running && <span className="badge running">● 监控中</span>}
        </div>
      </header>
      {isFirstRun && (
        <div className="onboarding-banner">
          首次使用：请先完成下方设置（采集源、提示词、模型服务），保存后即可开始监控。
        </div>
      )}
      <main className="app-main">
        {tab === "status" && (
          <StatusTab
            config={config}
            snapshot={snapshot}
            onStart={startMonitoring}
            onStop={stopMonitoring}
            onGoSettings={goSettings}
          />
        )}
        {tab === "settings" && (
          <SettingsTab
            config={config}
            gpu={gpu}
            firstRun={isFirstRun}
            onSave={saveConfig}
            onGoStatus={() => setTab("status")}
            onToast={showToast}
          />
        )}
      </main>
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
