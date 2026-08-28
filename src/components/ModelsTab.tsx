import { useState } from "react";
import { api } from "../api";
import type { Config, ConnectionTest, DetectionResult } from "../types";

interface Props {
  config: Config | null;
  onToast: (msg: string) => void;
  onPickModel: (model: string) => void;
  onGoSettings: () => void;
}

export default function ModelsTab({ config, onToast, onPickModel, onGoSettings }: Props) {
  const [testing, setTesting] = useState(false);
  const [conn, setConn] = useState<ConnectionTest | null>(null);
  const [detecting, setDetecting] = useState<"screen" | "camera" | null>(null);
  const [testResult, setTestResult] = useState<DetectionResult | null>(null);

  const doTestConnection = async () => {
    if (!config) return;
    setTesting(true);
    setConn(null);
    try {
      const r = await api.testConnection(config.modelApi.apiUrl, config.modelApi.apiKey);
      setConn(r);
      onToast(r.ok ? "连接成功" : `连接失败：${r.message}`);
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

  if (!config) return <div className="placeholder">加载中…</div>;

  return (
    <div className="models">
      <section className="card">
        <h2>当前模型服务</h2>
        {config.demoMode ? (
          <p className="hint">
            当前为<strong>演示模式</strong>，无需模型。如需真实检测，请到「设置」关闭演示模式并填写模型服务 URL。
          </p>
        ) : (
          <>
            <div className="row">
              <span className="gpu-name">🔌 {config.modelApi.apiUrl || "（未填写）"}</span>
              <span className="hint-inline">模型：{config.modelApi.model || "（未填写）"}</span>
            </div>
            <div className="row">
              <button className="btn" disabled={testing} onClick={doTestConnection}>
                {testing ? "测试中…" : "测试连接"}
              </button>
              <button className="btn" onClick={onGoSettings}>
                去设置
              </button>
            </div>
            {conn && (
              <div className={`test-result ${conn.ok ? "ok" : "warn"}`}>
                <p>
                  <strong>{conn.ok ? "✓ 连接成功" : "✗ 连接失败"}</strong>
                  {"　"}
                  {conn.message}
                </p>
                {conn.models.length > 0 && (
                  <>
                    <p className="hint">可用模型：</p>
                    <div className="model-chips">
                      {conn.models.map((m) => (
                        <button
                          key={m}
                          className="chip"
                          onClick={() => onPickModel(m)}
                          title="点击选用此模型"
                        >
                          {m}
                        </button>
                      ))}
                    </div>
                    <p className="hint">点上面任一模型可选用（记得去「设置」保存）。</p>
                  </>
                )}
              </div>
            )}
          </>
        )}
      </section>

      <section className="card">
        <h2>测试检测</h2>
        <p className="hint">
          使用当前「设置」中的提示词与模型，立即采集并检测一次（需要相应权限）。检测会调用模型服务，若未配置会在结果里给出原因。
        </p>
        <div className="row">
          <button
            className="btn"
            disabled={!!detecting}
            onClick={() => doTest("screen")}
          >
            {detecting === "screen" ? "检测中…" : "测试屏幕检测"}
          </button>
          <button
            className="btn"
            disabled={!!detecting}
            onClick={() => doTest("camera")}
          >
            {detecting === "camera" ? "检测中…" : "测试摄像头检测"}
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
