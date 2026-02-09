import { configApi } from '../api/tauri';
import { configureLatestRunTtlMs, DEFAULT_LATEST_RUN_TTL_MS } from '../stores/latestRun';
import {
  configureStalePlanRevToastCooldownMs,
  DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS,
} from './stalePlanRev';

const FRONTEND_CONFIG_KEYS = {
  latestRunTtlMs: 'latest_run_ttl_ms',
  stalePlanRevToastCooldownMs: 'stale_plan_rev_toast_cooldown_ms',
} as const;

export async function bootstrapFrontendRuntimeConfig(): Promise<void> {
  try {
    const configs = await configApi.listConfigs();
    const globalMap = new Map<string, string>();

    for (const item of configs) {
      if (String(item.scope_id || '').trim() !== 'global') continue;
      const key = String(item.key || '').trim();
      if (!key) continue;
      globalMap.set(key, String(item.value ?? '').trim());
    }

    configureLatestRunTtlMs(
      globalMap.get(FRONTEND_CONFIG_KEYS.latestRunTtlMs) ?? DEFAULT_LATEST_RUN_TTL_MS,
    );
    configureStalePlanRevToastCooldownMs(
      globalMap.get(FRONTEND_CONFIG_KEYS.stalePlanRevToastCooldownMs)
        ?? DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS,
    );
  } catch (error) {
    console.warn('前端运行治理配置加载失败，使用默认值:', error);
    configureLatestRunTtlMs(DEFAULT_LATEST_RUN_TTL_MS);
    configureStalePlanRevToastCooldownMs(DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS);
  }
}

