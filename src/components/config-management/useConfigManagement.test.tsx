import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/tauri', () => ({
  configApi: {
    listConfigs: vi.fn(),
    updateConfig: vi.fn(),
    getConfigSnapshot: vi.fn(),
    restoreFromSnapshot: vi.fn(),
  },
}));

vi.mock('../../stores/use-global-store', () => ({
  useCurrentUser: () => 'admin',
}));

vi.mock('../../hooks/useDebounce', () => ({
  useDebounce: <T,>(value: T) => value,
}));

vi.mock('../../services/frontendRuntimeConfig', () => ({
  bootstrapFrontendRuntimeConfig: vi.fn(),
}));

vi.mock('@tauri-apps/api/dialog', () => ({
  save: vi.fn(),
  open: vi.fn(),
}));

vi.mock('@tauri-apps/api/fs', () => ({
  writeTextFile: vi.fn(),
  readTextFile: vi.fn(),
}));

vi.mock('antd', () => ({
  message: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
  Modal: {
    confirm: vi.fn(),
  },
}));

import { configApi } from '../../api/tauri';
import { bootstrapFrontendRuntimeConfig } from '../../services/frontendRuntimeConfig';
import { open } from '@tauri-apps/api/dialog';
import { readTextFile } from '@tauri-apps/api/fs';
import { Modal } from 'antd';
import { useConfigManagement } from './useConfigManagement';

describe('useConfigManagement', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(configApi.listConfigs).mockResolvedValue([
      {
        scope_id: 'global',
        scope_type: 'GLOBAL',
        key: 'latest_run_ttl_ms',
        value: '120000',
      },
    ] as any);
    vi.mocked(configApi.updateConfig).mockResolvedValue(undefined as any);
    vi.mocked(configApi.restoreFromSnapshot).mockResolvedValue({ restored_count: 2 } as any);
    vi.mocked(open).mockResolvedValue('/tmp/config_snapshot.json' as any);
    vi.mocked(readTextFile).mockResolvedValue('{"global":{"latest_run_ttl_ms":"120000"}}' as any);
    vi.mocked(Modal.confirm).mockImplementation((options: any) => {
      void options?.onOk?.();
      return undefined as any;
    });
    vi.mocked(bootstrapFrontendRuntimeConfig).mockResolvedValue(undefined);
  });

  it('更新配置成功后应即时重载前端运行治理配置', async () => {
    const { result } = renderHook(() => useConfigManagement());

    await waitFor(() => {
      expect(configApi.listConfigs).toHaveBeenCalledTimes(1);
    });

    act(() => {
      result.current.handleEdit({
        scope_id: 'global',
        scope_type: 'GLOBAL',
        key: 'latest_run_ttl_ms',
        value: '120000',
      });
      result.current.setEditValue('180000');
      result.current.setUpdateReason('联调验证');
    });

    await act(async () => {
      await result.current.handleUpdate();
    });

    expect(configApi.updateConfig).toHaveBeenCalledWith(
      'global',
      'latest_run_ttl_ms',
      '180000',
      'admin',
      '联调验证',
    );
    expect(bootstrapFrontendRuntimeConfig).toHaveBeenCalledTimes(1);
  });

  it('快照恢复成功后应即时重载前端运行治理配置', async () => {
    const { result } = renderHook(() => useConfigManagement());

    await waitFor(() => {
      expect(configApi.listConfigs).toHaveBeenCalledTimes(1);
    });

    await act(async () => {
      await result.current.handleImportSnapshot();
    });

    await waitFor(() => {
      expect(configApi.restoreFromSnapshot).toHaveBeenCalledWith(
        '{"global":{"latest_run_ttl_ms":"120000"}}',
        'admin',
        '从快照恢复配置',
      );
    });

    expect(bootstrapFrontendRuntimeConfig).toHaveBeenCalledTimes(1);
  });
});
