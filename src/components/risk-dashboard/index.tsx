/**
 * 风险仪表盘 - 主组件
 *
 * 重构后：452 行 → ~70 行 (-85%)
 */

import React from 'react';
import { Alert, Col, Row, Spin, Typography } from 'antd';
import { useNavigate } from 'react-router-dom';
import NoActiveVersionGuide from '../NoActiveVersionGuide';

import { useRiskDashboard } from './useRiskDashboard';
import { DangerDayCard } from './DangerDayCard';
import { BlockedOrdersCard } from './BlockedOrdersCard';
import { ColdStockCard } from './ColdStockCard';
import { RollHealthCard } from './RollHealthCard';

const { Title } = Typography;

const RiskDashboard: React.FC = () => {
  const navigate = useNavigate();
  const state = useRiskDashboard();

  return (
    <div style={{ padding: 24 }}>
      <Title level={3} style={{ marginBottom: 24 }}>
        风险仪表盘
      </Title>

      {!state.activeVersionId && (
        <NoActiveVersionGuide
          title="尚无激活的排产版本"
          description="风险仪表盘需要一个激活的排产版本作为基础"
          onNavigateToPlan={() => navigate('/comparison')}
        />
      )}

      {state.activeVersionId && state.loadError && (
        <Alert
          type="error"
          showIcon
          message="数据加载失败"
          description={state.loadError}
          style={{ marginBottom: 16 }}
        />
      )}

      {state.activeVersionId && (
        <Spin spinning={state.loading} tip="加载中...">
          <Row gutter={[16, 16]}>
            {/* 危险日期卡片 */}
            <Col xs={24} sm={12} lg={8}>
              <DangerDayCard dangerDay={state.dangerDay} />
            </Col>

            {/* 阻塞紧急订单卡片 */}
            <Col xs={24} sm={12} lg={8}>
              <BlockedOrdersCard blockedOrders={state.blockedOrders} />
            </Col>

            {/* 冷库压力卡片 */}
            <Col xs={24} sm={12} lg={8}>
              <ColdStockCard coldStockBuckets={state.coldStockBuckets} />
            </Col>

            {/* 轧辊健康度卡片 */}
            {state.rollHealth.map((roll) => (
              <Col xs={24} sm={12} lg={12} key={roll.machineCode}>
                <RollHealthCard roll={roll} />
              </Col>
            ))}
          </Row>
        </Spin>
      )}
    </div>
  );
};

export default RiskDashboard;
