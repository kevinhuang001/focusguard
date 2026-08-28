import { useEffect, useState } from "react";
import { api } from "../api";
import { PRESET_MODELS, REMINDER_LABELS } from "../presets";
import type { Config, RecommendResult } from "../types";

interface Props {
  config: Config | null;
  gpu: RecommendResult | null;
  onSave: (cfg: Config) => Promise<void>;
  onToast: (msg: string) => void;
}

export default function SettingsTab({ config, gpu, onSave, onToast }: Props) {
  const [cfg, setCfg] = useState<Config | null>(null);
  const [monitors, setMonitors] = useState<string[]>([]);
  const [cameras, setCameras] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

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

  const set = <K extends keyof Config>(key: K, value: Config[K]) =>
    setCfg((c) => (c ? { ...c, [key]: value } : c));

  const setSource = (
    which: "screen" | "camera",
    patch: Partial<Config["screen"]>
  ) => setCfg((c) => (c ? { ...c, [which]: { ...c[which], ...patch } } : c));

  const setReminder = (patch: Partial<Config["reminder"]>) =>
    setCfg((c) => (c ? { ...c, reminder: { ...c.reminder, ...patch } } : c));

  const testReminder = () => {
    api
      .sendTestReminder(cfg.reminder.kind, cfg.reminder.voiceText)
      .then(() => onToast("已发送测试提醒"))
      .catch((e) => onToast(`测试失败：${e}`));
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
        <h2>检测参数</h2>
        <div className="row">
          <label>推理后端</label>
          <select
            value={cfg.backend}
            onChange={(e) => set("backend", e.target.value as Config["backend"])}
          >
            <option value="ollama">Ollama 本地模型</option>
            <option value="mock">模拟模式（无需模型，演示用）</option>
          </select>
        </div>
        {cfg.backend === "ollama" && (
          <>
            <div className="row">
              <label>模型</label>
              <input
                list="model-list"
                value={cfg.model}
                onChange={(e) => set("model", e.target.value)}
              />
              <datalist id="model-list">
                {PRESET_MODELS.map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </div>
            <div className="row">
              <label>Ollama 地址</label>
              <input
                value={cfg.ollamaUrl}
                onChange={(e) => set("ollamaUrl", e.target.value)}
              />
            </div>
          </>
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
        <h2>GPU 检测与参数推荐</h2>
        {gpu ? (
          <>
            <div className="row">
              <span className="gpu-name">🖥 {gpu.gpu.name || "未知 GPU"}</span>
              <span className="hint-inline">
                {gpu.gpu.vramMb
                  ? `${gpu.gpu.vramMb} MB 显存`
                  : "显存未知"}
                {" · "}
                {gpu.gpu.source}
              </span>
            </div>
            <div className="rec-box">
              <p>
                推荐模型：<strong>{gpu.model}</strong>　推荐间隔：
                <strong>{gpu.intervalSecs} 秒</strong>
              </p>
              <p className="hint">{gpu.note}</p>
              <button
                className="btn"
                onClick={() => {
                  set("model", gpu.model);
                  set("intervalSecs", gpu.intervalSecs);
                  onToast("已应用推荐参数，可再手动微调");
                }}
              >
                应用推荐参数
              </button>
            </div>
          </>
        ) : (
          <p className="hint">正在检测 GPU…（无 GPU 时会推荐 CPU 方案）</p>
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
                Linux 语音需要 speech-dispatcher 服务运行
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

      <div className="save-bar">
        <button
          className="btn primary big"
          disabled={saving}
          onClick={async () => {
            setSaving(true);
            try {
              await onSave(cfg);
            } catch (e) {
              onToast(`保存失败：${e}`);
            } finally {
              setSaving(false);
            }
          }}
        >
          {saving ? "保存中…" : "保存配置"}
        </button>
      </div>
    </div>
  );
}
