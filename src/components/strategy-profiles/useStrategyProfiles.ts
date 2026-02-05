/**
 * 策略配置面板状态管理 Hook
 */

import { useEffect, useMemo, useState } from 'react';
import { Form, message } from 'antd';
import { useNavigate } from 'react-router-dom';
import { configApi, planApi } from '../../api/tauri';
import { useCurrentUser } from '../../stores/use-global-store';
import type { CustomStrategyProfile, ModalMode, StrategyPresetRow } from './types';
import { BASE_STRATEGY_LABEL, suggestStrategyId } from './types';

export function useStrategyProfiles() {
  const navigate = useNavigate();
  const currentUser = useCurrentUser();

  const [loading, setLoading] = useState(false);
  const [presets, setPresets] = useState<StrategyPresetRow[]>([]);
  const [customProfiles, setCustomProfiles] = useState<CustomStrategyProfile[]>([]);

  const [modalOpen, setModalOpen] = useState(false);
  const [modalMode, setModalMode] = useState<ModalMode>('create');
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm();

  const presetsByKey = useMemo(() => {
    const map: Record<string, StrategyPresetRow> = {};
    (presets || []).forEach((p) => {
      map[String(p.strategy)] = p;
    });
    return map;
  }, [presets]);

  const baseStrategyOptions = useMemo(() => {
    const keys = Object.keys(presetsByKey);
    if (!keys.length) {
      return [
        { value: 'balanced', label: '均衡方案' },
        { value: 'urgent_first', label: '紧急优先' },
        { value: 'capacity_first', label: '产能优先' },
        { value: 'cold_stock_first', label: '冷料消化' },
      ];
    }
    return keys.map((k) => ({
      value: k,
      label: presetsByKey[k]?.title || BASE_STRATEGY_LABEL[k] || k,
    }));
  }, [presetsByKey]);

  const loadAll = async () => {
    setLoading(true);
    try {
      const [presetRes, customRes] = await Promise.all([
        planApi.getStrategyPresets().catch(() => null),
        configApi.listCustomStrategies().catch(() => null),
      ]);

      const nextPresets: StrategyPresetRow[] = Array.isArray(presetRes)
        ? presetRes
            .map((p: any) => ({
              strategy: String(p?.strategy ?? ''),
              title: String(p?.title ?? ''),
              description: String(p?.description ?? ''),
              default_parameters: p?.default_parameters ?? null,
            }))
            .filter((p) => p.strategy && p.title)
        : [];

      const nextCustom: CustomStrategyProfile[] = Array.isArray(customRes)
        ? customRes
            .map((p: any) => ({
              strategy_id: String(p?.strategy_id ?? ''),
              title: String(p?.title ?? ''),
              description: p?.description != null ? String(p.description) : null,
              base_strategy: String(p?.base_strategy ?? ''),
              parameters: p?.parameters ?? null,
            }))
            .filter((p) => p.strategy_id && p.title && p.base_strategy)
        : [];

      setPresets(nextPresets);
      setCustomProfiles(nextCustom);
    } catch (e: any) {
      message.error(e?.message || '加载策略配置失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const openCreate = (baseStrategy: string = 'balanced') => {
    setModalMode('create');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: suggestStrategyId(baseStrategy),
      title: '',
      description: '',
      base_strategy: baseStrategy,
      parameters: {},
      reason: '',
    });
    setModalOpen(true);
  };

  const openCopyFromPreset = (preset: StrategyPresetRow) => {
    setModalMode('copy');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: suggestStrategyId(preset.strategy),
      title: `${preset.title}（自定义）`,
      description: preset.description,
      base_strategy: preset.strategy,
      parameters: {},
      reason: '',
    });
    setModalOpen(true);
  };

  const openEdit = (profile: CustomStrategyProfile) => {
    setModalMode('edit');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: profile.strategy_id,
      title: profile.title,
      description: profile.description || '',
      base_strategy: profile.base_strategy,
      parameters: profile.parameters || {},
      reason: '',
    });
    setModalOpen(true);
  };

  const openCopyFromCustom = (profile: CustomStrategyProfile) => {
    setModalMode('copy');
    form.resetFields();
    form.setFieldsValue({
      strategy_id: suggestStrategyId(profile.base_strategy),
      title: `${profile.title}（复制）`,
      description: profile.description || '',
      base_strategy: profile.base_strategy,
      parameters: profile.parameters || {},
      reason: '',
    });
    setModalOpen(true);
  };

  const handleSave = async () => {
    const values = await form.validateFields();
    const reason = String(values?.reason || '').trim();
    if (!reason) {
      message.warning('请输入保存原因');
      return;
    }

    const payload: CustomStrategyProfile = {
      strategy_id: String(values?.strategy_id || '').trim(),
      title: String(values?.title || '').trim(),
      description: String(values?.description || '').trim() ? String(values.description).trim() : null,
      base_strategy: String(values?.base_strategy || '').trim(),
      parameters: values?.parameters || {},
    };

    setSaving(true);
    try {
      const resp = await configApi.saveCustomStrategy({
        strategy: payload,
        operator: currentUser || 'admin',
        reason,
      });
      message.success(resp?.message || '保存成功');
      setModalOpen(false);
      await loadAll();
    } catch (e: any) {
      message.error(e?.message || '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const closeModal = () => setModalOpen(false);

  return {
    // 状态
    loading,
    presets,
    presetsByKey,
    customProfiles,
    modalOpen,
    modalMode,
    saving,
    form,
    baseStrategyOptions,

    // 操作
    loadAll,
    openCreate,
    openCopyFromPreset,
    openEdit,
    openCopyFromCustom,
    handleSave,
    closeModal,
    navigate,
  };
}
