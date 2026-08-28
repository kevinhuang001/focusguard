import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Config,
  ConnectionTest,
  DetectionResult,
  GpuInfo,
  MonitorSnapshot,
  MonitorTick,
  PiperStatus,
  PiperVoice,
  RecommendResult,
  ReminderEvent,
  TtsConfig,
} from "./types";

export const api = {
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (cfg: Config) => invoke<void>("save_config", { cfg }),
  getGpuInfo: () => invoke<GpuInfo>("get_gpu_info"),
  getRecommendation: () => invoke<RecommendResult>("get_recommendation"),
  testConnection: (apiUrl: string, apiKey: string) =>
    invoke<ConnectionTest>("test_connection", { apiUrl, apiKey }),
  detectOnce: (source: string) => invoke<DetectionResult>("detect_once", { source }),
  startMonitoring: (cfg: Config) => invoke<void>("start_monitoring", { cfg }),
  stopMonitoring: () => invoke<void>("stop_monitoring"),
  getMonitorState: () => invoke<MonitorSnapshot>("get_monitor_state"),
  sendTestReminder: (kind: string, voiceText: string) =>
    invoke<string>("send_test_reminder", { kind, voiceText }),
  listMonitors: () => invoke<string[]>("list_monitors"),
  listCameras: () => invoke<string[]>("list_cameras"),
  readHistoryImage: (path: string) =>
    invoke<string>("read_history_image", { path }),
  listPiperVoices: () => invoke<PiperVoice[]>("list_piper_voices"),
  piperStatus: () => invoke<PiperStatus>("piper_status"),
  openPiperDownload: () => invoke<void>("open_piper_download"),
  downloadPiperVoice: (id: string) =>
    invoke<void>("download_piper_voice", { id }),
  ttsPreview: (tts: TtsConfig) => invoke<string>("tts_preview", { tts }),
};

export const events = {
  onTick: (cb: (t: MonitorTick) => void) =>
    listen<MonitorTick>("monitor://tick", (e) => cb(e.payload)),
  onState: (cb: (s: { running: boolean }) => void) =>
    listen<{ running: boolean }>("monitor://state", (e) => cb(e.payload)),
  onReminder: (cb: (r: ReminderEvent) => void) =>
    listen<ReminderEvent>("monitor://reminder", (e) => cb(e.payload)),
};
