import React from 'react';
import { Alert, Button, Space } from 'antd';
import { InfoCircleOutlined, SettingOutlined } from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import type { WorkbenchPathOverrideState } from '../../pages/workbench/types';

const WorkbenchAlerts: React.FC<{
  activeVersionId: string;
  pathOverride: WorkbenchPathOverrideState;
  onOpenPathOverrideCenter: () => void;
  onOpenPathOverrideConfirm: () => void;
  materialsIsLoading: boolean;
  materialsError: unknown;
  materialsCount: number;
  planItemsIsLoading: boolean;
  planItemsError: unknown;
  planItemsData: unknown;
}> = ({
  activeVersionId,
  pathOverride,
  onOpenPathOverrideCenter,
  onOpenPathOverrideConfirm,
  materialsIsLoading,
  materialsError,
  materialsCount,
  planItemsIsLoading,
  planItemsError,
  planItemsData,
}) => {
  const navigate = useNavigate();

  return (
    <>
      {pathOverride.pendingTotalCount > 0 && activeVersionId ? (
        <Alert
          type="warning"
          showIcon
          message={`路径规则待确认（跨日期/跨机组）：${pathOverride.pendingTotalCount} 条`}
          description={`范围 ${pathOverride.summaryRange.from} ~ ${pathOverride.summaryRange.to}（确认后建议重算生成新版本）`}
          action={
            <Space>
              <Button
                size="small"
                type="primary"
                icon={<InfoCircleOutlined />}
                loading={pathOverride.summaryIsFetching}
                onClick={onOpenPathOverrideCenter}
              >
                待确认中心
              </Button>
              <Button size="small" icon={<SettingOutlined />} onClick={() => navigate('/settings?tab=path_rule')}>
                路径规则设置
              </Button>
            </Space>
          }
        />
      ) : null}

      {pathOverride.pendingCount > 0 && pathOverride.context.machineCode && pathOverride.context.planDate ? (
        <Alert
          type="warning"
          showIcon
          message={`路径规则待确认：${pathOverride.pendingCount} 条`}
          description={`机组 ${pathOverride.context.machineCode} · 日期 ${pathOverride.context.planDate}（确认后建议重算生成新版本）`}
          action={
            <Space>
              <Button
                size="small"
                type="primary"
                icon={<InfoCircleOutlined />}
                loading={pathOverride.pendingIsFetching}
                onClick={onOpenPathOverrideConfirm}
              >
                去确认
              </Button>
              <Button size="small" icon={<SettingOutlined />} onClick={() => navigate('/settings?tab=path_rule')}>
                路径规则设置
              </Button>
            </Space>
          }
        />
      ) : null}

      {!materialsIsLoading && !materialsError && materialsCount === 0 ? (
        <Alert
          type="info"
          showIcon
          message="暂无物料数据"
          description="请先在“数据导入”导入材料数据文件；导入成功后再返回工作台进行排程与干预。"
          action={
            <Button size="small" type="primary" onClick={() => navigate('/import')}>
              去导入
            </Button>
          }
        />
      ) : null}

      {!planItemsIsLoading && !planItemsError && Array.isArray(planItemsData) && planItemsData.length === 0 ? (
        <Alert
          type="info"
          showIcon
          message="当前版本暂无排程明细"
          description="可点击右上角“一键优化”执行重算生成排程，然后再使用矩阵/甘特图视图进行调整。"
        />
      ) : null}
    </>
  );
};

export default React.memo(WorkbenchAlerts);
