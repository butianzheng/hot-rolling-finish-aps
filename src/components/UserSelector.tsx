import React from 'react';
import { Select, Space, message } from 'antd';
import { UserOutlined } from '@ant-design/icons';
import { useCurrentUser, useGlobalActions } from '../stores/use-global-store';

const { Option } = Select;

// 预设用户列表
const PRESET_USERS = [
  { value: 'admin', label: '系统管理员' },
  { value: 'dispatcher', label: '调度员' },
  { value: 'supervisor', label: '主管' },
  { value: 'operator', label: '操作员' },
];

const UserSelector: React.FC = () => {
  const currentUser = useCurrentUser();
  const { setCurrentUser } = useGlobalActions();

  const handleUserChange = (value: string) => {
    const user = PRESET_USERS.find((u) => u.value === value);
    setCurrentUser(value);
    message.success(`已切换到: ${user?.label}`);
  };

  return (
    <Space>
      <UserOutlined style={{ color: 'white', fontSize: 16 }} />
      <Select
        value={currentUser}
        onChange={handleUserChange}
        style={{ width: 130 }}
        size="small"
        popupMatchSelectWidth={130}
      >
        {PRESET_USERS.map((user) => (
          <Option key={user.value} value={user.value}>
            {user.label}
          </Option>
        ))}
      </Select>
    </Space>
  );
};

export default UserSelector;
