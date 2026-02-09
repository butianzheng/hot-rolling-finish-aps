/**
 * 配置管理状态管理 Hook
 */

import { useCallback, useEffect, useState } from 'react';
import { message, Modal } from 'antd';
import { configApi } from '../../api/tauri';
import { useCurrentUser } from '../../stores/use-global-store';
import { useDebounce } from '../../hooks/useDebounce';
import { bootstrapFrontendRuntimeConfig } from '../../services/frontendRuntimeConfig';
import { save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/api/fs';
import type { ConfigItem } from './types';

const DEFAULT_FRONTEND_RUNTIME_CONFIGS: Array<{ key: string; value: string }> = [
  { key: 'latest_run_ttl_ms', value: '120000' },
  { key: 'stale_plan_rev_toast_cooldown_ms', value: '4000' },
];

const FRONTEND_RUNTIME_CONFIG_RULES: Record<string, { min: number; max: number }> = {
  latest_run_ttl_ms: { min: 5_000, max: 15 * 60_000 },
  stale_plan_rev_toast_cooldown_ms: { min: 1_000, max: 60_000 },
};

function ensureGlobalConfigRows(configs: ConfigItem[]): ConfigItem[] {
  const existing = new Set(configs.map((item) => `${item.scope_id}::${item.key}`));
  const append: ConfigItem[] = [];

  for (const row of DEFAULT_FRONTEND_RUNTIME_CONFIGS) {
    const marker = `global::${row.key}`;
    if (existing.has(marker)) continue;
    append.push({
      scope_id: 'global',
      scope_type: 'GLOBAL',
      key: row.key,
      value: row.value,
    });
  }

  if (append.length === 0) return configs;
  return [...configs, ...append];
}

function validateConfigValue(config: ConfigItem, nextValue: string): string | null {
  const rule = FRONTEND_RUNTIME_CONFIG_RULES[config.key];
  if (!rule) return null;

  const parsed = Number(nextValue);
  if (!Number.isFinite(parsed)) {
    return `${config.key} 必须为数字`;
  }

  const rounded = Math.round(parsed);
  if (rounded < rule.min || rounded > rule.max) {
    return `${config.key} 必须在 ${rule.min} ~ ${rule.max} 之间`;
  }

  return null;
}

export interface UseConfigManagementReturn {
  // 加载状态
  loading: boolean;
  loadError: string | null;

  // 数据
  configs: ConfigItem[];
  filteredConfigs: ConfigItem[];
  scopeTypeCounts: Record<string, number>;

  // 筛选状态
  selectedScopeType: string;
  setSelectedScopeType: (type: string) => void;
  searchText: string;
  setSearchText: (text: string) => void;

  // 编辑模态框
  editModalVisible: boolean;
  editingConfig: ConfigItem | null;
  editValue: string;
  setEditValue: (value: string) => void;
  updateReason: string;
  setUpdateReason: (reason: string) => void;
  handleEdit: (record: ConfigItem) => void;
  handleUpdate: () => Promise<void>;
  closeEditModal: () => void;

  // 操作
  loadConfigs: () => Promise<void>;
  handleExportSnapshot: () => Promise<void>;
  handleImportSnapshot: () => Promise<void>;
  clearFilters: () => void;
}

export function useConfigManagement(): UseConfigManagementReturn {
  const currentUser = useCurrentUser();
  const [loading, setLoading] = useState(false);
  const [configs, setConfigs] = useState<ConfigItem[]>([]);
  const [filteredConfigs, setFilteredConfigs] = useState<ConfigItem[]>([]);
  const [selectedScopeType, setSelectedScopeType] = useState<string>('all');
  const [searchText, setSearchText] = useState<string>('');
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editModalVisible, setEditModalVisible] = useState(false);
  const [editingConfig, setEditingConfig] = useState<ConfigItem | null>(null);
  const [editValue, setEditValue] = useState('');
  const [updateReason, setUpdateReason] = useState('');

  // 防抖搜索文本（延迟 300ms）
  const debouncedSearchText = useDebounce(searchText, 300);

  // 加载配置
  const loadConfigs = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const result = ensureGlobalConfigRows(await configApi.listConfigs());
      setConfigs(result);
      setFilteredConfigs(result);
      message.success(`成功加载 ${result.length} 条配置`);
    } catch (error: any) {
      console.error('加载配置失败:', error);
      const msg = String(error?.message || error || '加载失败');
      setLoadError(msg);
      message.error(`加载失败: ${msg}`);
    } finally {
      setLoading(false);
    }
  }, []);

  // 编辑操作
  const handleEdit = useCallback((record: ConfigItem) => {
    setEditingConfig(record);
    setEditValue(record.value);
    setUpdateReason('');
    setEditModalVisible(true);
  }, []);

  const closeEditModal = useCallback(() => {
    setEditModalVisible(false);
  }, []);

  const handleUpdate = useCallback(async () => {
    if (!editingConfig) return;
    if (!updateReason.trim()) {
      message.warning('请输入修改原因');
      return;
    }

    const validationError = validateConfigValue(editingConfig, editValue);
    if (validationError) {
      message.warning(validationError);
      return;
    }

    setLoading(true);
    try {
      await configApi.updateConfig(
        editingConfig.scope_id,
        editingConfig.key,
        editValue,
        currentUser,
        updateReason
      );

      try {
        await bootstrapFrontendRuntimeConfig();
      } catch (reloadError) {
        console.warn('前端运行治理配置即时重载失败，将使用当前运行参数:', reloadError);
      }

      message.success('配置更新成功');
      setEditModalVisible(false);
      await loadConfigs();
    } catch (error: any) {
      console.error('更新配置失败:', error);
    } finally {
      setLoading(false);
    }
  }, [editingConfig, editValue, updateReason, currentUser, loadConfigs]);

  // 导出快照
  const handleExportSnapshot = useCallback(async () => {
    try {
      const snapshot = await configApi.getConfigSnapshot();
      const filePath = await save({
        defaultPath: `config_snapshot_${Date.now()}.json`,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (filePath) {
        await writeTextFile(filePath, JSON.stringify(snapshot, null, 2));
        message.success('配置快照导出成功');
      }
    } catch (error: any) {
      console.error('导出快照失败:', error);
    }
  }, []);

  // 导入快照
  const handleImportSnapshot = useCallback(async () => {
    try {
      const filePath = await open({
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (filePath && typeof filePath === 'string') {
        const content = await readTextFile(filePath);

        Modal.confirm({
          title: '确认恢复配置',
          content: '此操作将覆盖当前所有配置，是否继续？',
          onOk: async () => {
            setLoading(true);
            try {
              await configApi.restoreFromSnapshot(
                content,
                currentUser,
                '从快照恢复配置'
              );

              try {
                await bootstrapFrontendRuntimeConfig();
              } catch (reloadError) {
                console.warn('前端运行治理配置即时重载失败，将使用当前运行参数:', reloadError);
              }

              message.success('配置恢复成功');
              await loadConfigs();
            } catch (error: any) {
              console.error('恢复配置失败:', error);
            } finally {
              setLoading(false);
            }
          },
        });
      }
    } catch (error: any) {
      console.error('导入快照失败:', error);
    }
  }, [currentUser, loadConfigs]);

  // 筛选数据
  const filterData = useCallback(() => {
    let filtered = [...configs];

    // 按作用域类型筛选
    if (selectedScopeType !== 'all') {
      filtered = filtered.filter((item) => item.scope_type === selectedScopeType);
    }

    // 按搜索文本筛选（配置键或值）
    if (debouncedSearchText) {
      const searchLower = debouncedSearchText.toLowerCase();
      filtered = filtered.filter(
        (item) =>
          item.key.toLowerCase().includes(searchLower) ||
          item.value.toLowerCase().includes(searchLower) ||
          item.scope_id.toLowerCase().includes(searchLower)
      );
    }

    setFilteredConfigs(filtered);
  }, [configs, selectedScopeType, debouncedSearchText]);

  // 清除筛选
  const clearFilters = useCallback(() => {
    setSearchText('');
    setSelectedScopeType('all');
  }, []);

  // 计算作用域类型统计
  const scopeTypeCounts = configs.reduce((acc, config) => {
    acc[config.scope_type] = (acc[config.scope_type] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  // 初始加载
  useEffect(() => {
    loadConfigs();
  }, []);

  // 筛选条件变化时重新筛选
  useEffect(() => {
    filterData();
  }, [filterData]);

  return {
    loading,
    loadError,
    configs,
    filteredConfigs,
    scopeTypeCounts,
    selectedScopeType,
    setSelectedScopeType,
    searchText,
    setSearchText,
    editModalVisible,
    editingConfig,
    editValue,
    setEditValue,
    updateReason,
    setUpdateReason,
    handleEdit,
    handleUpdate,
    closeEditModal,
    loadConfigs,
    handleExportSnapshot,
    handleImportSnapshot,
    clearFilters,
  };
}
