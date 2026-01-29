/**
 * 主提示卡片组件
 */

import React from 'react';
import { Alert, Button, Card, Empty, Space, Typography } from 'antd';
import { AppstoreOutlined, UploadOutlined } from '@ant-design/icons';

const { Title, Text } = Typography;

export interface MainHintCardProps {
  title: string;
  description: string;
  onNavigateToPlan: () => void;
  onNavigateToImport?: () => void;
}

export const MainHintCard: React.FC<MainHintCardProps> = ({
  title,
  description,
  onNavigateToPlan,
  onNavigateToImport,
}) => {
  return (
    <Card style={{ textAlign: 'center', marginBottom: 24 }}>
      <Empty
        style={{ marginBottom: 24 }}
        description={
          <div>
            <Title level={4} style={{ marginBottom: 8 }}>
              {title}
            </Title>
            <Text type="secondary" style={{ fontSize: 14 }}>
              {description}
            </Text>
          </div>
        }
      />

      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Alert
          message="需要版本才能继续"
          description="当前系统中没有激活的排产版本。激活版本后，系统将自动加载相关的排产数据、材料信息、产能限制等关键信息。"
          type="warning"
          showIcon
          closable={false}
        />

        <Space wrap style={{ width: '100%', justifyContent: 'center' }}>
          {onNavigateToImport && (
            <Button
              type="primary"
              size="large"
              icon={<UploadOutlined />}
              onClick={onNavigateToImport}
              style={{ width: '100%', maxWidth: 300 }}
            >
              开始导入数据
            </Button>
          )}
          <Button
            type={onNavigateToImport ? 'default' : 'primary'}
            size="large"
            icon={<AppstoreOutlined />}
            onClick={onNavigateToPlan}
            style={{ width: '100%', maxWidth: 300 }}
          >
            前往版本管理/创建版本
          </Button>
        </Space>
      </Space>
    </Card>
  );
};

export default MainHintCard;
