import { useState } from "react";
import { api } from "../api";
import { PRESET_MODELS } from "../presets";
import type { DetectionResult, OllamaInfo } from "../types";

interface Props {
  ollama: OllamaInfo | null;
  refreshOllama: (url?: string) => Promise<OllamaInfo | null>;
  pullLog: string[];
  onToast: (msg: string) => void;
}

export default function ModelsTab({ ollama, refreshOllama, pullLog, onToast }: Props) {
  const [pullModel, setPullModel] = useState("");
  const [pulling, setPulling] = useState(false);
  const [testing, setTesting] = useState<"screen" | "camera" | null>(null);
  const [testResult, setTestResult] = useState<DetectionResult | null>(null);

  const doPull = async () => {
    const m = pullModel.trim();
    if (!m) {
      onToast("请输入要拉取的模型名");
      return;
    }
    setPulling(true);
    try {
      await api.pullModel(m);
      onToast(`模型 ${m} 拉取完成`);
      await refreshOllama();
    } catch (e) {
      onToast(`拉取失败：${e}`);
    } finally {
      setPulling(false);
    }
  };

  const doTest = async (source: "screen" | "camera") => {
    setTesting(source);
    setTestResult(null);
    try {
      setTestResult(await api.detectOnce(source));
      onToast("检测完成");
    } catch (e) {
      onToast(`检测失败：${e}`);
    } finally {
      setTesting(null);
    }
  };

  return (
    <div className="models">
      <section className="card">
        <h2>Ollama 状态</h2>
        <div className="row">
          <span className={`badge ${ollama?.installed ? "ok" : "warn"}`}>
            {ollama?.installed ? "已安装" : "未安装"}
          </span>
          <span className={`badge ${ollama?.running ? "ok" : "warn"}`}>
            {ollama?.running ? "运行中" : "未运行"}
          </span>
          {!ollama?.installed && (
            <span className="hint">
              请先安装 Ollama：
              <a href="https://ollama.com/download" target="_blank" rel="noreferrer">
                ollama.com/download
              </a>
            </span>
          )}
        </div>
        <div className="row">
          <button
            className="btn"
            onClick={async () => {
              try {
                await api.startOllama();
                onToast("Ollama 已后台启动，正在等待就绪…");
                setTimeout(() => refreshOllama(), 1500);
              } catch (e) {
                onToast(`启动失败：${e}`);
              }
            }}
          >
            启动 Ollama
          </button>
          <button className="btn" onClick={() => refreshOllama()}>
            刷新状态
          </button>
        </div>
      </section>

      <section className="card">
        <h2>拉取模型</h2>
        <div className="row">
          <input
            value={pullModel}
            onChange={(e) => setPullModel(e.target.value)}
            placeholder="例如 qwen2.5vl:3b"
            list="model-list2"
          />
          <datalist id="model-list2">
            {PRESET_MODELS.map((m) => (
              <option key={m} value={m} />
            ))}
          </datalist>
          <button className="btn primary" disabled={pulling} onClick={doPull}>
            {pulling ? "拉取中…" : "拉取模型"}
          </button>
        </div>
        {pullLog.length > 0 && <pre className="log">{pullLog.join("\n")}</pre>}
      </section>

      <section className="card">
        <h2>已安装模型</h2>
        {ollama?.models?.length ? (
          <table className="model-table">
            <thead>
              <tr>
                <th>模型</th>
                <th>大小</th>
                <th>修改时间</th>
              </tr>
            </thead>
            <tbody>
              {ollama.models.map((m) => (
                <tr key={m.name}>
                  <td>{m.name}</td>
                  <td>{(m.size / 1024 ** 3).toFixed(2)} GB</td>
                  <td>
                    {m.modifiedAt
                      ? new Date(m.modifiedAt).toLocaleString("zh-CN")
                      : "-"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="hint">
            暂无模型（或 Ollama 未运行）。请先拉取一个视觉模型，例如
            qwen2.5vl:3b，然后在「设置」中选中它。
          </p>
        )}
      </section>

      <section className="card">
        <h2>测试检测</h2>
        <p className="hint">
          使用当前「设置」中的提示词与模型，立即采集并检测一次（需要相应权限）。
        </p>
        <div className="row">
          <button
            className="btn"
            disabled={!!testing}
            onClick={() => doTest("screen")}
          >
            {testing === "screen" ? "检测中…" : "测试屏幕检测"}
          </button>
          <button
            className="btn"
            disabled={!!testing}
            onClick={() => doTest("camera")}
          >
            {testing === "camera" ? "检测中…" : "测试摄像头检测"}
          </button>
        </div>
        {testResult && (
          <div className={`test-result ${testResult.focused ? "ok" : "warn"}`}>
            <p>
              <strong>{testResult.focused ? "✓ 专注" : "✗ 开小差"}</strong>
              {"　"}
              {testResult.reason}
            </p>
            <p className="hint">
              模型：{testResult.model} · 耗时 {testResult.durationMs} ms · 来源：
              {testResult.source === "screen" ? "屏幕" : "摄像头"}
            </p>
          </div>
        )}
      </section>
    </div>
  );
}
