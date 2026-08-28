export interface SourceConfig {
  enabled: boolean;
  prompt: string;
  monitorIndex: number;
  cameraIndex: number;
}

export type ReminderKind = "none" | "system" | "voice" | "both";

export interface ReminderConfig {
  kind: ReminderKind;
  contentType: "fixed" | "ai";
  voiceText: string;
  cooldownSecs: number;
  missThreshold: number;
}

export interface TtsConfig {
  engine: "system" | "piper";
  systemVoice: string;
  piperVoice: string;
}

export interface ModelConfig {
  apiUrl: string;
  apiKey: string;
  model: string;
}

export interface Config {
  screen: SourceConfig;
  camera: SourceConfig;
  modelApi: ModelConfig;
  demoMode: boolean;
  intervalSecs: number;
  imageMaxWidth: number;
  tts: TtsConfig;
  reminder: ReminderConfig;
  configured: boolean;
}

export interface PiperVoice {
  id: string;
  label: string;
  lang: string;
  path: string;
}

export interface PiperStatus {
  engineInstalled: boolean;
  installedVoices: string[];
}

export interface GpuInfo {
  name: string;
  vramMb: number | null;
  source: string;
}

export interface RecommendResult {
  gpu: GpuInfo;
  model: string;
  intervalSecs: number;
  note: string;
}

export interface ConnectionTest {
  ok: boolean;
  message: string;
  models: string[];
}

export interface DetectionResult {
  focused: boolean;
  reason: string;
  source: string;
  model: string;
  backend: string;
  durationMs: number;
}

export interface MonitorTick {
  source: string;
  focused: boolean;
  reason: string;
  model: string;
  backend: string;
  durationMs: number;
  ts: number;
  error?: string;
  imagePath?: string;
}

export interface MonitorSnapshot {
  running: boolean;
  startedAt: number | null;
  lastTicks: MonitorTick[];
  missCount: number;
  lastReminderAt: number | null;
}

export interface ReminderEvent {
  kind: ReminderKind;
  title: string;
  text: string;
}
