import { useEffect, useState } from "react";
import { api } from "../api";
import { REMINDER_LABELS } from "../presets";
import type { Config, ConnectionTest, RecommendResult } from "../types";
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

  useEffect(() => {
    if (config) setCfg(structuredClone(config));
  }, [config]);

  useEffect(() => {
    api
      .listMonitors()
      .then(setMonitors)
      .catch(() => onToast("无法获取显示器列表（Linux Wayland 下不支持）"));
    api
      .listCameras()
      .then(setCameras)
      .catch(() => onToast("无法枚举摄像头（可能无设备或无权限）"));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!cfg) return <div className="placeholder">加载中…</div>;

  const validation = validateConfig(cfg);

  const set = <K extends keyof Config>(key: K, value: Config[K]) =>
    setCfg((c) => (c ? { ...c, [key]: value } : c));
  const setModel = (patch: Partial<Config["modelApi"]>) =>
    setCfg((c) => (c ? { ...c, modelApi: { ...c.modelApi, ...patch } } : c));
  const setSource = (
    which: "screen" | "camera",
    patch: Partial<Config["screen"]>
  ) => setCfg((c) => (c ? { ...c, [which]: { ...c[which], ...patch } } : c));
  const setReminder = (patch: Partial<Config["reminder"]>) =>
    setCfg((c) => (c ? { ...c, reminder: { ...c.reminder, ...patch } } : c));

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

  const testReminder = () => {
    api
      .sendTestReminder(cfg.reminder.kind, cfg.reminder.voiceText)
      .then(() => onToast("已发送测试提醒"))
      .catch((e) => onToast(`测试失败：${e}`));
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

  return (
    <div className="settings">
      <section className="card">
        <h2>采集源</h2>
        <div className="row">
          <label className="check">
            <input
              type="checkbox"
              checked={cfg.screen.enabled}
              onChange={(e) => setSource("screen", { enabled: e.target.checked })}
            />
            屏幕画面
          </label>
          <select
            value={cfg.screen.monitorIndex}
            disabled={!cfg.screen.enabled}
            onChange={(e) =>
              setSource("screen", { monitorIndex: Number(e.target.value) })
            }
          >
            {(monitors.length ? monitors : ["#0 默认显示器"]).map((m, i) => (
              <option key={i} value={i}>
                {m}
              </option>
            ))}
          </select>
        </div>
        <textarea
          rows={3}
          placeholder="描述你的专注任务，例如：写代码推进当前开发任务，不要浏览社交媒体、看视频或聊天。"
          value={cfg.screen.prompt}
          disabled={!cfg.screen.enabled}
          onChange={(e) => setSource("screen", { prompt: e.target.value })}
        />
        <div className="row">
          <label className="check">
            <input
              type="checkbox"
              checked={cfg.camera.enabled}
              onChange={(e) =>
                setSource("camera", { enabled: e.target.checked })
              }
            />
            摄像头画面
          </label>
          <select
            value={cfg.camera.cameraIndex}
            disabled={!cfg.camera.enabled}
            onChange={(e) =>
              setSource("camera", { cameraIndex: Number(e.target.value) })
            }
          >
            {(cameras.length ? cameras : ["#0 默认摄像头"]).map((m, i) => (
              <option key={i} value={i}>
                {m}
              </option>
            ))}
          </select>
        </div>
        <textarea
          rows={3}
          placeholder="描述专注状态，例如：专注地看着屏幕工作，不要玩手机、东张西望或离开座位。"
          value={cfg.camera.prompt}
          disabled={!cfg.camera.enabled}
          onChange={(e) => setSource("camera", { prompt: e.target.value })}
        />
        <p className="hint">
          屏幕与摄像头可同时开启，各自使用独立的提示词。至少开启一个采集源。
        </p>
      </section>

      <section className="card">
        <h2>模型服务（OpenAI 兼容）</h2>
        <div className="row">
          <label className="check">
            <input
              type="checkbox"
              checked={cfg.demoMode}
              onChange={(e) => set("demoMode", e.target.checked)}
            />
            演示模式（无需模型，模拟检测）
          </label>
        </div>
        {!cfg.demoMode ? (
          <>
            <div className="row">
              <label>服务 URL</label>
              <input
                placeholder="http://localhost:11434/v1 或 https://api.openai.com/v1"
                value={cfg.modelApi.apiUrl}
                onChange={(e) => setModel({ apiUrl: e.target.value })}
              />
            </div>
            <div className="row">
              <label>API Key</label>
              <input
                type="password"
                placeholder="本地服务可留空"
                value={cfg.modelApi.apiKey}
                onChange={(e) => setModel({ apiKey: e.target.value })}
              />
            </div>
            <div className="row">
              <label>模型名</label>
              <input
                placeholder="例如 qwen3-vl:4b 或 gpt-4o-mini"
                value={cfg.modelApi.model}
                onChange={(e) => setModel({ model: e.target.value })}
              />
              <span className="hint-inline">
                需要支持图像输入
                {gpu ? `（已按 GPU 推荐：${gpu.model}）` : ""}
              </span>
            </div>
            <div className="row">
              <button className="btn" disabled={testing} onClick={runTestConnection}>
                {testing ? "测试中…" : "测试连接"}
              </button>
              {connTest && (
                <span className={`hint-inline ${connTest.ok ? "link-btn" : "warn-text"}`}>
                  {connTest.message}
                  {connTest.models.length > 0 && `：${connTest.models.join(", ")}`}
                </span>
              )}
            </div>
            <p className="hint">
              兼容任意 OpenAI 风格服务。若是本地 Ollama，需先自行启动并拉取模型，再把地址填成
              <code> http://localhost:11434/v1</code>（应用不负责下载/管理模型）。
            </p>
          </>
        ) : (
          <p className="hint">
            演示模式：无需模型即可体验完整监控流程（检测结果由程序模拟，约每 8 轮出现一次开小差）。
          </p>
        )}
        <div className="row">
          <label>检测间隔（秒）</label>
          <input
            type="number"
            min={2}
            max={3600}
            value={cfg.intervalSecs}
            onChange={(e) => set("intervalSecs", Number(e.target.value))}
          />
          <span className="hint-inline">
            {gpu ? `已按 GPU 自动推荐 ${gpu.intervalSecs}s` : ""}，可手动改
          </span>
        </div>
        <div className="row">
          <label>图片最大宽度（px）</label>
          <input
            type="number"
            min={160}
            max={1920}
            value={cfg.imageMaxWidth}
            onChange={(e) => set("imageMaxWidth", Number(e.target.value))}
          />
          <span className="hint-inline">越小推理越快、越省显存</span>
        </div>
      </section>

      <section className="card">
        <h2>GPU 检测</h2>
        {gpu ? (
          <div className="row">
            <span className="gpu-name">🖥 {gpu.gpu.name || "未知 GPU"}</span>
            <span className="hint-inline">
              {gpu.gpu.vramMb ? `${gpu.gpu.vramMb} MB 显存` : "显存未知"}
              {" · "}
              {gpu.gpu.source}
            </span>
            <span className="hint-inline">{gpu.note}</span>
          </div>
        ) : (
          <p className="hint">正在检测 GPU…</p>
        )}
      </section>

      <section className="card">
        <h2>开小差提醒</h2>
        <div className="row radios">
          {(["none", "system", "voice", "both"] as const).map((k) => (
            <label key={k} className="check">
              <input
                type="radio"
                checked={cfg.reminder.kind === k}
                onChange={() => setReminder({ kind: k })}
              />
              {REMINDER_LABELS[k]}
            </label>
          ))}
        </div>
        {cfg.reminder.kind !== "none" && (
          <>
            <div className="row">
              <label>语音内容</label>
              <input
                value={cfg.reminder.voiceText}
                disabled={cfg.reminder.kind === "system"}
                onChange={(e) => setReminder({ voiceText: e.target.value })}
              />
            </div>
            <div className="row">
              <button className="btn" onClick={testReminder}>
                测试提醒
              </button>
              <span className="hint-inline">
                Windows 走系统语音（SAPI），Linux 需要 speech-dispatcher
              </span>
            </div>
          </>
        )}
        <div className="row">
          <label>连续开小差 N 次后提醒</label>
          <input
            type="number"
            min={1}
            max={20}
            value={cfg.reminder.missThreshold}
            onChange={(e) =>
              setReminder({ missThreshold: Number(e.target.value) })
            }
          />
        </div>
        <div className="row">
          <label>提醒冷却（秒）</label>
          <input
            type="number"
            min={5}
            max={3600}
            value={cfg.reminder.cooldownSecs}
            onChange={(e) =>
              setReminder({ cooldownSecs: Number(e.target.value) })
            }
          />
        </div>
      </section>

      {!validation.ok && (
        <div className="validation-errors">
          <p>配置还不完整，无法开始监控：</p>
          <ul>
            {validation.errors.map((e, i) => (
              <li key={i}>· {e}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="save-bar">
        <button
          className="btn primary big"
          disabled={saving || !validation.ok}
          onClick={doSave}
        >
          {firstRun ? "保存配置并进入监控" : saving ? "保存中…" : "保存配置"}
        </button>
      </div>
    </div>
  );
}
