import React from 'react';
import { Empty, Button, Card, Row, Col, Steps, Space, Alert, Divider, Typography, theme } from 'antd';
import {
  AppstoreOutlined,
  CheckCircleOutlined,
  NumberOutlined,
  ThunderboltOutlined,
  UploadOutlined,
} from '@ant-design/icons';

const { Title, Text } = Typography;

interface NoActiveVersionGuideProps {
  onNavigateToPlan: () => void; // 导航到排产方案的回调
  onNavigateToImport?: () => void; // 导航到数据导入的回调（可选）
  title?: string; // 自定义标题
  description?: string; // 自定义描述
}

const NoActiveVersionGuide: React.FC<NoActiveVersionGuideProps> = ({
  onNavigateToPlan,
  onNavigateToImport,
  title = '尚无激活的排产版本',
  description = '请先创建并激活一个排产版本，才能进行排产和调度操作',
}) => {
  const { token } = theme.useToken();

  const steps = [
    ...(onNavigateToImport
      ? [
          {
            title: '导入数据',
            description: '先导入材料/订单等基础数据（支持冲突处理与导入历史）',
            icon: <UploadOutlined />,
          },
        ]
      : []),
    {
      title: '创建排产方案',
      description: '在"排产方案"页面中点击"创建方案"按钮，输入方案名称',
      icon: <AppstoreOutlined />,
    },
    {
      title: '创建版本',
      description: '在方案中点击"创建版本"按钮，设置窗口天数',
      icon: <NumberOutlined />,
    },
    {
      title: '激活版本',
      description: '点击版本右侧的"激活"按钮，激活排产版本',
      icon: <CheckCircleOutlined />,
    },
    {
      title: '开始排产',
      description: '返回当前页面，系统会自动加载排产数据',
      icon: <ThunderboltOutlined />,
    },
  ];

  return (
    <div style={{ padding: '40px 20px' }}>
      <Row justify="center" gutter={[24, 24]}>
        <Col xs={24} sm={22} md={20} lg={16} xl={14}>
          {/* 主要提示区域 */}
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

          {/* 操作步骤指南 */}
          <Card
            title={
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span>快速开始指南</span>
              </div>
            }
            variant="borderless"
          >
            <Steps
              direction="vertical"
              current={-1}
              items={steps.map((step) => ({
                title: (
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                    <span style={{ fontSize: 16 }}>{step.icon}</span>
                    <strong style={{ fontSize: 14 }}>{step.title}</strong>
                  </div>
                ),
                description: (
                  <p style={{ margin: '8px 0 0 28px', color: token.colorTextSecondary, fontSize: 13 }}>
                    {step.description}
                  </p>
                ),
              }))}
            />

            <Divider style={{ margin: '24px 0' }} />

            {/* 提示信息 */}
            <div
              style={{
                background: token.colorFillAlter,
                padding: 12,
                borderRadius: token.borderRadiusLG,
              }}
            >
              <div style={{ marginBottom: 8 }}>
                <strong>✨ 说明：</strong>
              </div>
              <ul style={{ margin: 0, paddingLeft: 20, lineHeight: 1.8 }}>
                <li>一个方案可以包含多个版本，用于不同的排产方案对比</li>
                <li>同一时刻只能有一个版本处于"激活"状态</li>
                <li>激活版本后，所有排产和调度操作都基于该版本进行</li>
                <li>版本激活后，可随时切换到其他版本，无需重复创建</li>
              </ul>
            </div>
          </Card>

          {/* 常见问题 */}
          <Card
            title={<div>❓ 常见问题</div>}
            variant="borderless"
            style={{ marginTop: 24 }}
          >
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <div>
                <p style={{ marginBottom: 4 }}>
                  <strong>Q: 为什么看不到排产数据？</strong>
                </p>
                <p style={{ margin: '0 0 8px 16px', color: token.colorTextSecondary }}>
                  A: 系统需要一个激活的排产版本作为基础。没有激活版本时，所有依赖版本的功能都会显示此引导页面。
                </p>
              </div>

              <div>
                <p style={{ marginBottom: 4 }}>
                  <strong>Q: 如何切换到其他版本？</strong>
                </p>
                <p style={{ margin: '0 0 8px 16px', color: token.colorTextSecondary }}>
                  A: 在"排产方案"页面中，选择要激活的版本，点击"激活"按钮即可。系统会自动应用新版本的数据。
                </p>
              </div>

              <div>
                <p style={{ marginBottom: 4 }}>
                  <strong>Q: 激活版本会影响已有数据吗？</strong>
                </p>
                <p style={{ margin: '0 0 8px 16px', color: token.colorTextSecondary }}>
                  A: 不会。激活版本只是改变当前工作版本，不会删除或修改任何已有数据。
                </p>
              </div>
            </Space>
          </Card>
        </Col>
      </Row>
    </div>
  );
};

export default NoActiveVersionGuide;
