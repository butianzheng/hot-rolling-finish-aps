import { describe, expect, it, vi, beforeEach } from 'vitest';
import { bootstrapFrontendRuntimeConfig } from './frontendRuntimeConfig';
import * as latestRunModule from '../stores/latestRun';
import * as stalePlanRevModule from './stalePlanRev';

// Mock configApi
vi.mock('../api/tauri', () => ({
  configApi: {
    listConfigs: vi.fn(),
  },
}));

describe('frontendRuntimeConfig', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('bootstrapFrontendRuntimeConfig', () => {
    it('应该成功加载配置并应用', async () => {
      const { configApi } = await import('../api/tauri');
      const configureLatestRunTtlMsSpy = vi.spyOn(latestRunModule, 'configureLatestRunTtlMs');
      const configureStalePlanRevToastCooldownMsSpy = vi.spyOn(
        stalePlanRevModule,
        'configureStalePlanRevToastCooldownMs'
      );

      (configApi.listConfigs as any).mockResolvedValue([
        { scope_id: 'global', key: 'latest_run_ttl_ms', value: '60000' },
        { scope_id: 'global', key: 'stale_plan_rev_toast_cooldown_ms', value: '30000' },
      ]);

      await bootstrapFrontendRuntimeConfig();

      expect(configureLatestRunTtlMsSpy).toHaveBeenCalledWith('60000');
      expect(configureStalePlanRevToastCooldownMsSpy).toHaveBeenCalledWith('30000');
    });

    it('应该过滤非 global scope 的配置', async () => {
      const { configApi } = await import('../api/tauri');
      const configureLatestRunTtlMsSpy = vi.spyOn(latestRunModule, 'configureLatestRunTtlMs');

      (configApi.listConfigs as any).mockResolvedValue([
        { scope_id: 'user', key: 'latest_run_ttl_ms', value: '60000' },
        { scope_id: 'global', key: 'latest_run_ttl_ms', value: '90000' },
      ]);

      await bootstrapFrontendRuntimeConfig();

      // 应该只使用 global scope 的配置
      expect(configureLatestRunTtlMsSpy).toHaveBeenCalledWith('90000');
    });

    it('应该处理空配置列表', async () => {
      const { configApi } = await import('../api/tauri');
      const configureLatestRunTtlMsSpy = vi.spyOn(latestRunModule, 'configureLatestRunTtlMs');
      const configureStalePlanRevToastCooldownMsSpy = vi.spyOn(
        stalePlanRevModule,
        'configureStalePlanRevToastCooldownMs'
      );

      (configApi.listConfigs as any).mockResolvedValue([]);

      await bootstrapFrontendRuntimeConfig();

      // 应该使用默认值
      expect(configureLatestRunTtlMsSpy).toHaveBeenCalledWith(
        latestRunModule.DEFAULT_LATEST_RUN_TTL_MS
      );
      expect(configureStalePlanRevToastCooldownMsSpy).toHaveBeenCalledWith(
        stalePlanRevModule.DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS
      );
    });

    it('应该处理配置加载失败', async () => {
      const { configApi } = await import('../api/tauri');
      const configureLatestRunTtlMsSpy = vi.spyOn(latestRunModule, 'configureLatestRunTtlMs');
      const configureStalePlanRevToastCooldownMsSpy = vi.spyOn(
        stalePlanRevModule,
        'configureStalePlanRevToastCooldownMs'
      );
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      (configApi.listConfigs as any).mockRejectedValue(new Error('加载失败'));

      await bootstrapFrontendRuntimeConfig();

      // 应该使用默认值
      expect(configureLatestRunTtlMsSpy).toHaveBeenCalledWith(
        latestRunModule.DEFAULT_LATEST_RUN_TTL_MS
      );
      expect(configureStalePlanRevToastCooldownMsSpy).toHaveBeenCalledWith(
        stalePlanRevModule.DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS
      );
      expect(consoleWarnSpy).toHaveBeenCalled();

      consoleWarnSpy.mockRestore();
    });

    it('应该处理空 key 的配置项', async () => {
      const { configApi } = await import('../api/tauri');
      const configureLatestRunTtlMsSpy = vi.spyOn(latestRunModule, 'configureLatestRunTtlMs');

      (configApi.listConfigs as any).mockResolvedValue([
        { scope_id: 'global', key: '', value: '60000' },
        { scope_id: 'global', key: null, value: '70000' },
        { scope_id: 'global', key: 'latest_run_ttl_ms', value: '90000' },
      ]);

      await bootstrapFrontendRuntimeConfig();

      // 应该只使用有效 key 的配置
      expect(configureLatestRunTtlMsSpy).toHaveBeenCalledWith('90000');
    });

    it('应该处理 null/undefined 值', async () => {
      const { configApi } = await import('../api/tauri');
      const configureLatestRunTtlMsSpy = vi.spyOn(latestRunModule, 'configureLatestRunTtlMs');

      (configApi.listConfigs as any).mockResolvedValue([
        { scope_id: 'global', key: 'latest_run_ttl_ms', value: null },
      ]);

      await bootstrapFrontendRuntimeConfig();

      // 应该将 null 转换为空字符串
      expect(configureLatestRunTtlMsSpy).toHaveBeenCalledWith('');
    });

    it('应该处理部分配置缺失', async () => {
      const { configApi } = await import('../api/tauri');
      const configureLatestRunTtlMsSpy = vi.spyOn(latestRunModule, 'configureLatestRunTtlMs');
      const configureStalePlanRevToastCooldownMsSpy = vi.spyOn(
        stalePlanRevModule,
        'configureStalePlanRevToastCooldownMs'
      );

      (configApi.listConfigs as any).mockResolvedValue([
        { scope_id: 'global', key: 'latest_run_ttl_ms', value: '60000' },
        // 缺少 stale_plan_rev_toast_cooldown_ms
      ]);

      await bootstrapFrontendRuntimeConfig();

      expect(configureLatestRunTtlMsSpy).toHaveBeenCalledWith('60000');
      // 缺失的配置应该使用默认值
      expect(configureStalePlanRevToastCooldownMsSpy).toHaveBeenCalledWith(
        stalePlanRevModule.DEFAULT_STALE_PLAN_REV_TOAST_COOLDOWN_MS
      );
    });
  });
});
