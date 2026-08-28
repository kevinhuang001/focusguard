import { useEffect, useState } from "react";
import { api, events } from "../api";
import { REMINDER_LABELS } from "../presets";
import type {
  Config,
  ConnectionTest,
  DetectionResult,
  PiperStatus,
  PiperVoice,
  RecommendResult,
} from "../types";
import { validateConfig } from "../validate";

interface Props {
  config: Config | null;
  gpu: RecommendResult | null;
  firstRun: boolean;
  onSave: (cfg: Config) => Promise<void>;
  onGoStatus: () => void;
  onToast: (msg: string) => void;
}

export default function SettingsTab({
  config,
  gpu,
  firstRun,
  onSave,
  onGoStatus,
  onToast,
}: Props) {
  const [cfg, setCfg] = useState<Config | null>(null);
  const [monitors, setMonitors] = useState<string[]>([]);
  const [cameras, setCameras] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [connTest, setConnTest] = useState<ConnectionTest | null>(null);
  const [testing, setTesting] = useState(false);
  const [detecting, setDetecting] = useState<"screen" | "camera" | null>(null);
  const [testResult, setTestResult] = useState<DetectionResult | null>(null);
  const [piperVoices, setPiperVoices] = useState<PiperVoice[]>([]);
  const [piper, setPiper] = useState<PiperStatus | null>(null);
  const [ttsTesting, setTtsTesting] = useState(false);
  const [dl, setDl] = useState<{ id: string; pct: number } | null>(null);

  useEffect(() => {
    if (config) setCfg(structuredClone(config));
  }, [config]);

  useEffect(() => {
    api.listMonitors().then(setMonitors).catch(() => onToast("无法获取显示器列表"));
    api.listCameras().then(setCameras).catch(() => onToast("无法枚举摄像头"));
    api.listPiperVoices().then(setPiperVoices).catch(() => {});
    api.piperStatus().then(setPiper).catch(() => {});
    const un = events.onDownloadProgress((p) => {
      if (p.total && p.total > 0) {
        setDl({ id: p.id, pct: Math.round((p.bytes / p.total) * 100) });
      } else {
        setDl((d) => (d ? { ...d, pct: Math.min(d.pct + 1, 99) } : { id: p.id, pct: 5 }));
      }
    });
    return () => { un.then((f) => f()); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!cfg) return <div className="placeholder">加载中…</div>;

  const validation = validateConfig(cfg);

  const set = <K extends keyof Config>(key: K, value: Config[K]) =>
    setCfg((c) => (c ? { ...c, [key]: value } : c));
  const setModel = (patch: Partial<Config["modelApi"]>) =>
    setCfg((c) => (c ? { ...c, modelApi: { ...c.modelApi, ...patch } } : c));
  const setSource = (which: "screen" | "camera", patch: Partial<Config["screen"]>) =>
    setCfg((c) => (c ? { ...c, [which]: { ...c[which], ...patch } } : c));
  const setReminder = (patch: Partial<Config["reminder"]>) =>
    setCfg((c) => (c ? { ...c, reminder: { ...c.reminder, ...patch } } : c));
  const setTts = (patch: Partial<Config["tts"]>) =>
    setCfg((c) => (c ? { ...c, tts: { ...c.tts, ...patch } } : c));

  const runTestConnection = async () => {
    setTesting(true);
    setConnTest(null);
    try {
      const r = await api.testConnection(cfg.modelApi.apiUrl, cfg.modelApi.apiKey);
      setConnTest(r);
      onToast(r.ok ? r.message : `连接失败：${r.message}`);
    } catch (e) {
      onToast(`测试出错：${e}`);
    } finally {
      setTesting(false);
    }
  };

  const doTest = async (source: "screen" | "camera") => {
    setDetecting(source);
    setTestResult(null);
    try {
      setTestResult(await api.detectOnce(source));
      onToast("检测完成");
    } catch (e) {
      onToast(`检测失败：${e}`);
    } finally {
      setDetecting(null);
    }
  };

  const doTtsPreview = async () => {
    setTtsTesting(true);
    try {
      const msg = await api.ttsPreview(cfg.tts);
      onToast(msg);
    } catch (e) {
      onToast(`试听失败：${e}`);
    } finally {
      setTtsTesting(false);
    }
  };

  const doPiperVoice = async (id: string) => {
    setDl({ id, pct: 0 });
    try {
      await api.downloadPiperVoice(id);
      setPiper(await api.piperStatus());
      onToast("音色下载完成，可试听");
    } catch (e) {
      onToast(`下载失败：${e}`);
    } finally {
      setDl(null);
    }
  };

  const testReminder = async () => {
    try {
      const msg = await api.sendTestReminder(cfg.reminder.kind, cfg.reminder.voiceText);
      onToast(msg);
    } catch (e) {
      onToast(`测试失败：${e}`);
    }
  };

  const doSave = async () => {
    if (!validation.ok) {
      onToast(validation.errors[0]);
      return;
    }
    setSaving(true);
    try {
      await onSave(cfg);
      if (firstRun) onGoStatus();
    } catch (e) {
      onToast(`保存失败：${e}`);
    } finally {
      setSaving(false);
    }
  };

  const voice = piperVoices.find((v) => v.id === cfg.tts.piperVoice);
  const voiceInstalled = piper?.installedVoices.includes(cfg.tts.piperVoice) ?? false;

  return (
    <div className="settings">
      <section className="card">
        <h2>采集源</h2>
        <div className="row">
          <label className="check">
            <input type="checkbox" checked={cfg.screen.enabled} onChange={(e) => setSource("screen", { enabled: e.target.checked })} />
            屏幕画面
          </label>
          <select
            value={cfg.screen.monitorIndex}
            disabled={!cfg.screen.enabled}
            onChange={(e) => setSource("screen", { monitorIndex: Number(e.target.value) })}
          >
            {(monitors.length ? monitors : ["#0 默认显示器"]).map((m, i) => (
              <option key={i} value={i}>{m}</option>
            ))}
          </select>
        </div>
        <textarea rows={3} placeholder="描述你的专注任务，例如：写代码推进当前开发任务，不要浏览社交媒体、看视频或聊天。" value={cfg.screen.prompt} disabled={!cfg.screen.enabled} onChange={(e) => setSource("screen", { prompt: e.target.value })} />
        <div className="row">
          <label className="check">
            <input type="checkbox" checked={cfg.camera.enabled} onChange={(e) => setSource("camera", { enabled: e.target.checked })} />
            摄像头画面
          </label>
          <select value={cfg.camera.cameraIndex} disabled={!cfg.camera.enabled} onChange={(e) => setSource("camera", { cameraIndex: Number(e.target.value) })}>
            {(cameras.length ? cameras : ["#0 默认摄像头"]).map((m, i) => (
              <option key={i} value={i}>{m}</option>
            ))}
          </select>
        </div>
        <textarea rows={3} placeholder="描述专注状态，例如：专注地看着屏幕工作，不要玩手机、东张西望或离开座位。" value={cfg.camera.prompt} disabled={!cfg.camera.enabled} onChange={(e) => setSource("camera", { prompt: e.target.value })} />
      </section>

      <section className="card">
        <h2>模型服务（OpenAI 兼容）</h2>
        <div className="row">
          <label className="check">
            <input type="checkbox" checked={cfg.demoMode} onChange={(e) => set("demoMode", e.target.checked)} />
            演示模式（无需模型，模拟检测）
          </label>
        </div>
        {!cfg.demoMode ? (
          <>
            <div className="row">
              <label>服务 URL</label>
              <input placeholder="http://localhost:11434/v1 或 https://api.openai.com/v1" value={cfg.modelApi.apiUrl} onChange={(e) => setModel({ apiUrl: e.target.value })} />
            </div>
            <div className="row">
              <label>API Key</label>
              <input type="password" placeholder="本地服务可留空" value={cfg.modelApi.apiKey} onChange={(e) => setModel({ apiKey: e.target.value })} />
            </div>
            <div className="row">
              <label>模型名</label>
              <input placeholder="例如 qwen3-vl:4b 或 gpt-4o-mini" value={cfg.modelApi.model} onChange={(e) => setModel({ model: e.target.value })} />
              <span className="hint-inline">需支持图像输入{gpu ? `（已按 GPU 推荐：${gpu.model}）` : ""}</span>
            </div>
            <div className="row">
              <button className="btn" disabled={testing} onClick={runTestConnection}>{testing ? "测试中…" : "测试连接"}</button>
              <button className="btn" disabled={!!detecting} onClick={() => doTest("screen")}>{detecting === "screen" ? "检测中…" : "测试屏幕检测"}</button>
              <button className="btn" disabled={!!detecting} onClick={() => doTest("camera")}>{detecting === "camera" ? "检测中…" : "测试摄像头检测"}</button>
            </div>
            {connTest && (
              <div className={`test-result ${connTest.ok ? "ok" : "warn"}`}>
                <p><strong>{connTest.ok ? "✓ 连接成功" : "✗ 连接失败"}</strong>　{connTest.message}</p>
                {connTest.models.length > 0 && <p className="hint">可用模型：{connTest.models.join(", ")}</p>}
              </div>
            )}
            {testResult && (
              <div className={`test-result ${testResult.focused ? "ok" : "warn"}`}>
                <p><strong>{testResult.focused ? "✓ 专注" : "✗ 开小差"}</strong>　{testResult.reason}</p>
                <p className="hint">模型：{testResult.model} · {testResult.durationMs} ms · {testResult.source === "screen" ? "屏幕" : "摄像头"}</p>
              </div>
            )}
            <p className="hint">兼容任意 OpenAI 风格服务。本地 Ollama 需自行启动并拉取模型，把地址填成 http://localhost:11434/v1（应用不负责下载/管理模型）。</p>
          </>
        ) : (
          <p className="hint">演示模式：无需模型即可体验完整监控流程（检测结果由程序模拟，约每 8 轮出现一次开小差）。</p>
        )}
      </section>

      <section className="card">
        <h2>检测参数</h2>
        <div className="row">
          <label>检测间隔（秒）</label>
          <input type="number" min={2} max={3600} value={cfg.intervalSecs} onChange={(e) => set("intervalSecs", Number(e.target.value))} />
          <span className="hint-inline">{gpu ? `已按 GPU 自动推荐 ${gpu.intervalSecs}s` : ""}，可手动改</span>
        </div>
        <div className="row">
          <label>图片最大宽度（px）</label>
          <input type="number" min={160} max={1920} value={cfg.imageMaxWidth} onChange={(e) => set("imageMaxWidth", Number(e.target.value))} />
          <span className="hint-inline">越小推理越快、越省显存</span>
        </div>
        {gpu && (
          <div className="row">
            <span className="gpu-name">🖥 {gpu.gpu.name || "未知 GPU"}</span>
            <span className="hint-inline">{gpu.gpu.vramMb ? `${gpu.gpu.vramMb} MB 显存` : "显存未知"} · {gpu.gpu.source}</span>
            <span className="hint-inline">{gpu.note}</span>
          </div>
        )}
      </section>

      <section className="card">
        <h2>语音（TTS）</h2>
        <div className="row">
          <label>引擎</label>
          <select value={cfg.tts.engine} onChange={(e) => setTts({ engine: e.target.value as "system" | "piper" })}>
            <option value="system">系统语音（SAPI / say / spd-say）</option>
            <option value="piper">Piper（本地开源，三平台一致）</option>
          </select>
          <button className="btn" disabled={ttsTesting} onClick={doTtsPreview}>{ttsTesting ? "试听中…" : "试听"}</button>
        </div>
        {cfg.tts.engine === "system" ? (
          <div className="row">
            <label>系统音色</label>
            <input value={cfg.tts.systemVoice} placeholder="留空用默认语音" onChange={(e) => setTts({ systemVoice: e.target.value })} />
          </div>
        ) : (
          <>
            <div className="row">
              <label>音色</label>
              <select value={cfg.tts.piperVoice} onChange={(e) => setTts({ piperVoice: e.target.value })}>
                {piperVoices.map((v) => (
                  <option key={v.id} value={v.id}>{v.label}（{v.id}）</option>
                ))}
              </select>
            </div>
            <div className="row">
              {voice && (
                <button className="btn" disabled={voiceInstalled || !!dl} onClick={() => doPiperVoice(voice.id)}>
                  {voiceInstalled ? "✓ 已下载" : dl ? "下载中…" : "下载此音色"}
                </button>
              )}
              <span className="hint-inline">
                {piper?.engineInstalled ? "Piper 引擎已就绪" : "Piper 引擎未安装，" }
                {!piper?.engineInstalled && (
                  <a className="link-btn" onClick={() => api.openPiperDownload().then(() => onToast("已打开下载页"))}>
                    打开 Piper 下载页
                  </a>
                )}
              </span>
            </div>
            {dl && (
              <div className="dl-row">
                <div className="progress"><div className="progress-bar" style={{ width: `${dl.pct}%` }} /></div>
                <span className="hint-inline">{dl.pct}%</span>
              </div>
            )}
            <p className="hint">Piper 是本地开源 TTS。需先把引擎放到应用数据目录的 tts/ 文件夹并下载音色，提醒音色三平台一致、支持中文。</p>
          </>
        )}
      </section>

      <section className="card">
        <h2>开小差提醒</h2>
        <div className="row">
          <label>提醒内容</label>
          <select value={cfg.reminder.contentType} onChange={(e) => setReminder({ contentType: e.target.value as "fixed" | "ai" })}>
            <option value="fixed">固定文案</option>
            <option value="ai">AI 生成（基于画面判断）</option>
          </select>
        </div>
        {cfg.reminder.contentType === "fixed" && (
          <div className="row">
            <label>文案</label>
            <input value={cfg.reminder.voiceText} onChange={(e) => setReminder({ voiceText: e.target.value })} />
          </div>
        )}
        <div className="row radios">
          {(["none", "system", "voice", "both"] as const).map((k) => (
            <label key={k} className="check">
              <input type="radio" checked={cfg.reminder.kind === k} onChange={() => setReminder({ kind: k })} />
              {REMINDER_LABELS[k]}
            </label>
          ))}
        </div>
        {cfg.reminder.kind !== "none" && (
          <div className="row">
            <button className="btn" onClick={testReminder}>测试提醒</button>
          </div>
        )}
        <div className="row">
          <label>连续开小差 N 次后提醒</label>
          <input type="number" min={1} max={20} value={cfg.reminder.missThreshold} onChange={(e) => setReminder({ missThreshold: Number(e.target.value) })} />
        </div>
        <div className="row">
          <label>提醒冷却（秒）</label>
          <input type="number" min={5} max={3600} value={cfg.reminder.cooldownSecs} onChange={(e) => setReminder({ cooldownSecs: Number(e.target.value) })} />
        </div>
      </section>

      {!validation.ok && (
        <div className="validation-errors">
          <p>配置还不完整，无法开始监控：</p>
          <ul>{validation.errors.map((e, i) => <li key={i}>· {e}</li>)}</ul>
        </div>
      )}

      <div className="save-bar">
        <button className="btn primary big" disabled={saving || !validation.ok} onClick={doSave}>
          {firstRun ? "保存配置并进入监控" : saving ? "保存中…" : "保存配置"}
        </button>
      </div>
    </div>
  );
}
