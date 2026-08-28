export interface SourceConfig {
  enabled: boolean;
  prompt: string;
  monitorIndex: number;
  cameraIndex: number;
}

export type ReminderKind = "none" | "system" | "voice" | "both";

export interface ReminderConfig {
  kind: ReminderKind;
  voiceText: string;
  cooldownSecs: number;
  missThreshold: number;
}

export interface Config {
  screen: SourceConfig;
  camera: SourceConfig;
  backend: "ollama" | "mock";
  model: string;
  intervalSecs: number;
  imageMaxWidth: number;
  ollamaUrl: string;
  reminder: ReminderConfig;
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

export interface OllamaModel {
  name: string;
  size: number;
  digest: string;
  modifiedAt: string;
}

export interface OllamaInfo {
  installed: boolean;
  running: boolean;
  models: OllamaModel[];
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

export interface PullEvent {
  model: string;
  line: string;
}
