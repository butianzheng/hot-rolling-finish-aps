// ==========================================
// 风险仪表盘 - Risk & Impact Dashboard
// ==========================================
// Bento Box 风格的风险卡片网格
// ==========================================

import React, { useState, useEffect } from 'react';
import { Row, Col, Card, Statistic, List, Progress, Empty, Typography, Space, Tag, Spin, Alert } from 'antd';
import {
  WarningOutlined,
  FireOutlined,
  InboxOutlined,
  ToolOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons';
import { ScatterChart, Scatter, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { FONT_FAMILIES } from '../theme';
import { useNavigate } from 'react-router-dom';
import NoActiveVersionGuide from './NoActiveVersionGuide';
import { useActiveVersionId } from '../stores/use-global-store';
import { dashboardApi } from '../api/tauri';
import { decisionService } from '../services/decision-service';
import { parseAlertLevel } from '../types/decision';
import type {
  DangerDayData,
  RollCampaignHealth,
} from '../types/dashboard';

const { Title, Text } = Typography;

type BlockedUrgentOrderRow = {
  contract_no: string;
  due_date: string;
  urgency_level: string;
  fail_type: string;
  completion_rate: number;
  days_to_due: number;
  machine_code: string;
};

type ColdStockBucketRow = {
  machine_code: string;
  age_bin: string;
  pressure_level: string;
  count: number;
  weight_t: number;
  avg_age_days: number;
  max_age_days: number;
};

const RiskDashboard: React.FC = () => {
  const navigate = useNavigate();
  const activeVersionId = useActiveVersionId();
  const [dangerDay, setDangerDay] = useState<DangerDayData | null>(null);
  const [blockedOrders, setBlockedOrders] = useState<BlockedUrgentOrderRow[]>([]);
  const [coldStockBuckets, setColdStockBuckets] = useState<ColdStockBucketRow[]>([]);
  const [rollHealth, setRollHealth] = useState<RollCampaignHealth[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    if (!activeVersionId) return;

    setLoading(true);
    setLoadError(null);

    (async () => {
      try {
        const [mostRiskyRes, urgentRes, coldRes, rollRes] = await Promise.allSettled([
          dashboardApi.getMostRiskyDate(activeVersionId),
          dashboardApi.getUnsatisfiedUrgentMaterials(activeVersionId),
          dashboardApi.getColdStockMaterials(activeVersionId, 30),
          decisionService.getAllRollCampaignAlerts(activeVersionId),
        ]);

        const errors: string[] = [];

        if (mostRiskyRes.status === 'fulfilled') {
          const most = mostRiskyRes.value?.items?.[0];
          if (most) {
            const riskLevelRaw = String(most.risk_level || '').toUpperCase();
            const riskLevel: DangerDayData['riskLevel'] =
              riskLevelRaw === 'CRITICAL'
                ? 'critical'
                : riskLevelRaw === 'HIGH'
                ? 'high'
                : riskLevelRaw === 'MEDIUM'
                ? 'medium'
                : 'low';

            setDangerDay({
              date: most.plan_date,
              riskLevel,
              capacityOverflow: Number(most.overload_weight_t || 0),
              urgentBacklog: Number(most.urgent_failure_count || 0),
              reasons: (most.top_reasons || []).map((r: any) => r?.msg || '').filter(Boolean),
            });
          } else {
            setDangerDay(null);
          }
        } else {
          errors.push('危险日期加载失败');
          setDangerDay(null);
        }

        if (urgentRes.status === 'fulfilled') {
          setBlockedOrders((urgentRes.value?.items || []) as BlockedUrgentOrderRow[]);
        } else {
          errors.push('紧急阻塞订单加载失败');
          setBlockedOrders([]);
        }

        if (coldRes.status === 'fulfilled') {
          setColdStockBuckets((coldRes.value?.items || []) as ColdStockBucketRow[]);
        } else {
          errors.push('库龄/冷料压库加载失败');
          setColdStockBuckets([]);
        }

        if (rollRes.status === 'fulfilled') {
          const rollItems = rollRes.value?.items || [];
          const mappedRollHealth: RollCampaignHealth[] = rollItems.map((r) => {
            const status = parseAlertLevel(String(r.alertLevel || ''));
            const mappedStatus: RollCampaignHealth['status'] =
              status === 'HARD_STOP'
                ? 'critical'
                : status === 'WARNING' || status === 'SUGGEST'
                ? 'warning'
                : 'healthy';

            return {
              machineCode: r.machineCode,
              currentTonnage: r.currentTonnageT,
              threshold: r.hardLimitT,
              status: mappedStatus,
              estimatedRollsRemaining: 0, // 后端暂无“件数”口径，这里仅保留字段以兼容旧 UI
            };
          });
          setRollHealth(mappedRollHealth);
        } else {
          errors.push('换辊警报加载失败');
          setRollHealth([]);
        }

        setLoadError(errors.length ? errors.join('；') : null);
      } catch (e: any) {
        console.error('[RiskDashboard] load failed:', e);
        setLoadError(e?.message || '数据加载失败');
      } finally {
        setLoading(false);
      }
    })();
  }, [activeVersionId]);

  // 获取风险等级颜色
  const getRiskColor = (level: string) => {
    switch (level) {
      case 'critical':
        return '#ff4d4f';
      case 'high':
        return '#faad14';
      case 'medium':
        return '#1677ff';
      default:
        return '#52c41a';
    }
  };

  // 获取轧辊状态颜色
  const getRollStatusColor = (status: string) => {
    switch (status) {
      case 'critical':
        return '#ff4d4f';
      case 'warning':
        return '#faad14';
      default:
        return '#52c41a';
    }
  };

  return (
    <div style={{ padding: 24 }}>
      <Title level={3} style={{ marginBottom: 24 }}>
        风险仪表盘
      </Title>

      {!activeVersionId && (
        <NoActiveVersionGuide
          title="尚无激活的排产版本"
          description="风险仪表盘需要一个激活的排产版本作为基础"
          onNavigateToPlan={() => navigate('/comparison')}
        />
      )}

      {activeVersionId && loadError && (
        <Alert
          type="error"
          showIcon
          message="数据加载失败"
          description={loadError}
          style={{ marginBottom: 16 }}
        />
      )}

      {activeVersionId && (
        <Spin spinning={loading} tip="加载中...">
      <Row gutter={[16, 16]}>
        {/* 危险日期卡片 */}
        <Col xs={24} sm={12} lg={8}>
          <Card
            hoverable
            style={{
              height: '100%',
              borderLeft: `4px solid ${dangerDay ? getRiskColor(dangerDay.riskLevel) : '#52c41a'}`,
            }}
          >
            <Space direction="vertical" style={{ width: '100%' }} size={16}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <Title level={5} style={{ margin: 0 }}>
                  <WarningOutlined style={{ marginRight: 8, color: getRiskColor(dangerDay?.riskLevel || 'low') }} />
                  危险日期
                </Title>
                {dangerDay && (
                  <Tag color={getRiskColor(dangerDay.riskLevel)}>
                    {dangerDay.riskLevel.toUpperCase()}
                  </Tag>
                )}
              </div>

              {dangerDay ? (
                <>
                  <Statistic
                    title="最高风险日"
                    value={dangerDay.date}
                    valueStyle={{
                      fontSize: 24,
                      fontWeight: 'bold',
                      color: getRiskColor(dangerDay.riskLevel),
                      fontFamily: FONT_FAMILIES.MONOSPACE,
                    }}
                  />
                  <div>
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      风险原因:
                    </Text>
                    <List
                      size="small"
                      dataSource={dangerDay.reasons}
                      renderItem={(item) => (
                        <List.Item style={{ padding: '4px 0', border: 'none' }}>
                          <Text style={{ fontSize: 13 }}>• {item}</Text>
                        </List.Item>
                      )}
                    />
                  </div>
                </>
              ) : (
                <Empty description="暂无风险" image={Empty.PRESENTED_IMAGE_SIMPLE} />
              )}
            </Space>
          </Card>
        </Col>

        {/* 阻塞紧急订单卡片 */}
        <Col xs={24} sm={12} lg={8}>
          <Card hoverable style={{ height: '100%' }}>
            <Space direction="vertical" style={{ width: '100%' }} size={16}>
              <Title level={5} style={{ margin: 0 }}>
                <FireOutlined style={{ marginRight: 8, color: '#ff4d4f' }} />
                阻塞紧急订单
              </Title>

              {blockedOrders.length > 0 ? (
                <>
                  <Statistic
                    title="阻塞数量"
                    value={blockedOrders.length}
                    suffix="件"
                    valueStyle={{ color: '#ff4d4f' }}
                  />
                  <List
                    size="small"
                    dataSource={blockedOrders.slice(0, 3)}
                    renderItem={(item) => (
                      <List.Item style={{ padding: '8px 0' }}>
                        <Space direction="vertical" size={4} style={{ width: '100%' }}>
                          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                            <Text strong style={{ fontFamily: FONT_FAMILIES.MONOSPACE, fontSize: 13 }}>
                              {item.contract_no}
                            </Text>
                            <Tag color={item.urgency_level === 'L3' ? '#ff4d4f' : '#faad14'}>
                              {item.urgency_level}
                            </Tag>
                          </div>
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            <ClockCircleOutlined /> 距交期 {item.days_to_due} 天 - {item.fail_type}
                          </Text>
                        </Space>
                      </List.Item>
                    )}
                  />
                </>
              ) : (
                <Empty description="无阻塞订单" image={Empty.PRESENTED_IMAGE_SIMPLE} />
              )}
            </Space>
          </Card>
        </Col>

        {/* 冷库压力卡片 */}
        <Col xs={24} sm={12} lg={8}>
          <Card hoverable style={{ height: '100%' }}>
            <Space direction="vertical" style={{ width: '100%' }} size={16}>
              <Title level={5} style={{ margin: 0 }}>
                <InboxOutlined style={{ marginRight: 8, color: '#13c2c2' }} />
                冷库压力
              </Title>

              {coldStockBuckets.length > 0 ? (
                <>
                  <Statistic
                    title="超期材料"
                    value={coldStockBuckets.reduce((sum, b) => sum + (b.count || 0), 0)}
                    suffix="件"
                    valueStyle={{ color: '#13c2c2' }}
                  />
                  <div style={{ height: 180 }}>
                    <ResponsiveContainer width="100%" height="100%">
                      <ScatterChart margin={{ top: 10, right: 10, bottom: 10, left: 10 }}>
                        <CartesianGrid strokeDasharray="3 3" />
                        <XAxis
                          type="number"
                          dataKey="avg_age_days"
                          name="平均库龄"
                          label={{ value: '平均库龄(天)', position: 'insideBottom', offset: -5 }}
                        />
                        <YAxis
                          type="number"
                          dataKey="weight_t"
                          name="重量(t)"
                          label={{ value: '重量(t)', angle: -90, position: 'insideLeft' }}
                        />
                        <Tooltip
                          cursor={{ strokeDasharray: '3 3' }}
                          content={({ active, payload }) => {
                            if (active && payload && payload.length) {
                              const data = payload[0].payload as ColdStockBucketRow;
                              return (
                                <div
                                  style={{
                                    backgroundColor: 'rgba(0, 0, 0, 0.8)',
                                    padding: '8px 12px',
                                    borderRadius: 4,
                                    color: '#fff',
                                  }}
                                >
                                  <div style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>{data.machine_code}</div>
                                  <div>库龄分桶: {data.age_bin}</div>
                                  <div>压力等级: {data.pressure_level}</div>
                                  <div>数量: {data.count}</div>
                                  <div>重量: {data.weight_t} 吨</div>
                                  <div>平均库龄: {data.avg_age_days} 天</div>
                                </div>
                              );
                            }
                            return null;
                          }}
                        />
                        <Scatter data={coldStockBuckets} fill="#13c2c2">
                          {coldStockBuckets.map((entry, index) => (
                            <Cell
                              key={`cell-${index}`}
                              fill={
                                entry.pressure_level === 'CRITICAL'
                                  ? '#ff4d4f'
                                  : entry.pressure_level === 'HIGH'
                                  ? '#faad14'
                                  : '#13c2c2'
                              }
                            />
                          ))}
                        </Scatter>
                      </ScatterChart>
                    </ResponsiveContainer>
                  </div>
                </>
              ) : (
                <Empty description="无冷库压力" image={Empty.PRESENTED_IMAGE_SIMPLE} />
              )}
            </Space>
          </Card>
        </Col>

        {/* 轧辊健康度卡片 */}
        {rollHealth.map((roll) => (
          <Col xs={24} sm={12} lg={12} key={roll.machineCode}>
            <Card hoverable>
              <Space direction="vertical" style={{ width: '100%' }} size={16}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Title level={5} style={{ margin: 0 }}>
                    <ToolOutlined style={{ marginRight: 8, color: getRollStatusColor(roll.status) }} />
                    轧辊健康度 - {roll.machineCode}
                  </Title>
                  <Tag color={getRollStatusColor(roll.status)}>
                    {roll.status.toUpperCase()}
                  </Tag>
                </div>

                <div>
                  <Text type="secondary">当前吨位</Text>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginTop: 4 }}>
                    <Text
                      strong
                      style={{
                        fontSize: 32,
                        fontFamily: FONT_FAMILIES.MONOSPACE,
                        color: getRollStatusColor(roll.status),
                      }}
                    >
                      {roll.currentTonnage}
                    </Text>
                    <Text type="secondary">/ {roll.threshold} 吨</Text>
                  </div>
                </div>

                <Progress
                  percent={(roll.currentTonnage / roll.threshold) * 100}
                  strokeColor={getRollStatusColor(roll.status)}
                  status={roll.status === 'critical' ? 'exception' : 'active'}
                  format={(percent) => `${percent?.toFixed(1)}%`}
                />

                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <Statistic
                    title="距离更换"
                    value={roll.threshold - roll.currentTonnage}
                    suffix="吨"
                    valueStyle={{ fontSize: 20, color: getRollStatusColor(roll.status) }}
                  />
                </div>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>
        </Spin>
      )}
    </div>
  );
};

export default RiskDashboard;
