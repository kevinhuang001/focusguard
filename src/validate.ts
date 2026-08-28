import type { Config } from "./types";

export interface ValidationResult {
  ok: boolean;
  errors: string[];
}

/** 配置结构完整性校验（不检查模型服务连通性，那是后端的事） */
export function validateConfig(cfg: Config): ValidationResult {
  const errors: string[] = [];

  if (!cfg.screen.enabled && !cfg.camera.enabled) {
    errors.push("请至少开启一个采集源（屏幕或摄像头）");
  }
  if (cfg.screen.enabled && cfg.screen.prompt.trim().length === 0) {
    errors.push("请填写屏幕监控的提示词，说明你要专注的任务");
  }
  if (cfg.camera.enabled && cfg.camera.prompt.trim().length === 0) {
    errors.push("请填写摄像头监控的提示词，说明专注状态");
  }
  if (!cfg.demoMode) {
    if (cfg.modelApi.apiUrl.trim().length === 0) {
      errors.push("请填写模型服务 URL（OpenAI 兼容地址，如 http://localhost:11434/v1）");
    }
    if (cfg.modelApi.model.trim().length === 0) {
      errors.push("请填写要使用的模型名");
    }
  }
  if (cfg.intervalSecs < 2 || cfg.intervalSecs > 3600) {
    errors.push("检测间隔需在 2~3600 秒之间");
  }

  return { ok: errors.length === 0, errors };
}
