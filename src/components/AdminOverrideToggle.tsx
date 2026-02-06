// ==========================================
// 管理员覆盖模式切换组件
// ==========================================
// 允许管理员临时覆盖 红线保护
// 使用警告样式提醒用户风险
// ==========================================

import React from 'react';
import { Switch, Tooltip, Space } from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import { useAdminOverrideMode, useGlobalActions } from '../stores/use-global-store';

export const AdminOverrideToggle: React.FC = () => {
  const adminOverrideMode = useAdminOverrideMode();
  const { setAdminOverrideMode } = useGlobalActions();

  return (
    <Tooltip
      title={
        adminOverrideMode
          ? '管理员覆盖模式已启用，可绕过红线保护'
          : '启用管理员覆盖模式，允许绕过冻结区和温度检查保护'
      }
    >
      <Space size={8}>
        {adminOverrideMode && (
          <WarningOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
        )}
        <Switch
          checked={adminOverrideMode}
          onChange={setAdminOverrideMode}
          checkedChildren="覆盖"
          unCheckedChildren="保护"
          style={{
            backgroundColor: adminOverrideMode ? '#ff4d4f' : undefined,
          }}
        />
      </Space>
    </Tooltip>
  );
};
