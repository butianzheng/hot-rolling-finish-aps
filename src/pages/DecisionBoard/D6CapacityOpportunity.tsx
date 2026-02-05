// ==========================================
// D6决策：容量优化机会页面
// ==========================================
// 职责: 展示容量优化机会，识别未充分利用的产能，提供负载平衡建议
// ==========================================

import React, { useState, useMemo } from 'react';
import { Card, Row, Col, Statistic, Tag, Spin, Alert, Progress, Space, Select, Table, theme } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  CalendarOutlined,
  RiseOutlined,
  SwapOutlined,
} from '@ant-design/icons';
import { useRecentDaysCapacityOpportunity } from '../../hooks/queries/use-decision-queries';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import { useActiveVersionId } from '../../stores/use-global-store';
import type { CapacityOpportunity, OpportunityType } from '../../types/decision';
import {
  OPPORTUNITY_TYPE_COLORS,
  OPPORTUNITY_TYPE_LABELS,
  isUnderutilized,
  getUtilizationColor,
} from '../../types/decision/d6-capacity-opportunity';

// 辅助函数：解析机会类型字符串为 OpportunityType
function parseOpportunityType(typeStr: string): OpportunityType {
  const upper = typeStr.toUpperCase().replace(/-/g, '_');
  if (['UNDERUTILIZED', 'MOVABLE_LOAD', 'STRUCTURE_FIX', 'URGENT_INSERTION', 'LOAD_BALANCE'].includes(upper)) {
    return upper as OpportunityType;
  }
  return 'UNDERUTILIZED';
}

// ==========================================
// 主组件
// ==========================================

interface D6CapacityOpportunityProps {
  embedded?: boolean;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
}

export const D6CapacityOpportunity: React.FC<D6CapacityOpportunityProps> = ({ embedded, onOpenDrilldown }) => {
  const { token } = theme.useToken();
  const warningBg = (token as any).colorWarningBg ?? token.colorFillTertiary;
  const warningBgHover = (token as any).colorWarningBgHover ?? token.colorFillSecondary;
  const versionId = useActiveVersionId();
  const [selectedDays, setSelectedDays] = useState<number>(30);
  const effectiveDays = embedded ? 30 : selectedDays;
  const openWithDrawer = !!embedded && !!onOpenDrilldown;

  // 获取容量优化机会数据
  const { data, isLoading, error } = useRecentDaysCapacityOpportunity(versionId, effectiveDays);

  // 计算统计数据
  const stats = useMemo(() => {
    if (!data?.items || data.items.length === 0) {
      return {
        totalOpportunities: 0,
        underutilizedCount: 0,
        totalOpportunitySpace: 0,
        avgCurrentUtilization: 0,
        avgOptimizedUtilization: 0,
      };
    }

    const totalOpportunities = data.summary?.totalOpportunities ?? data.totalCount ?? 0;
    const underutilizedCount = data.items.filter((item) =>
      isUnderutilized(item.currentUtilPct)
    ).length;
    const totalOpportunitySpace = data.summary?.totalOpportunitySpaceT ?? 0;
    const avgCurrentUtilization = data.summary?.avgCurrentUtilPct ?? 0;
    const avgOptimizedUtilization = data.summary?.avgOptimizedUtilPct ?? 0;

    return {
      totalOpportunities,
      underutilizedCount,
      totalOpportunitySpace,
      avgCurrentUtilization,
      avgOptimizedUtilization,
    };
  }, [data]);

  // 表格列定义
  const columns: ColumnsType<CapacityOpportunity> = [
    {
      title: '排产日期',
      dataIndex: 'planDate',
      key: 'planDate',
      width: 120,
      fixed: 'left',
      sorter: (a, b) => a.planDate.localeCompare(b.planDate),
    },
    {
      title: '机组',
      dataIndex: 'machineCode',
      key: 'machineCode',
      width: 100,
      filters: Array.from(
        new Set((data?.items || []).map((item) => item.machineCode))
      ).map((code) => ({
        text: code,
        value: code,
      })),
      onFilter: (value, record) => record.machineCode === value,
      render: (code: string) => <Tag color="blue">{code}</Tag>,
    },
    {
      title: '机会类型',
      dataIndex: 'opportunityType',
      key: 'opportunityType',
      width: 140,
      render: (typeStr: string) => {
        const type = parseOpportunityType(typeStr);
        return (
          <Tag color={OPPORTUNITY_TYPE_COLORS[type]} style={{ margin: 0 }}>
            {OPPORTUNITY_TYPE_LABELS[type]}
          </Tag>
        );
      },
    },
    {
      title: '当前利用率',
      dataIndex: 'currentUtilPct',
      key: 'currentUtilPct',
      width: 130,
      render: (pct: number) => (
        <div>
          <div style={{ fontSize: '12px', marginBottom: '4px' }}>{pct.toFixed(1)}%</div>
          <Progress
            percent={pct}
            size="small"
            showInfo={false}
            strokeColor={getUtilizationColor(pct)}
          />
        </div>
      ),
      sorter: (a, b) => a.currentUtilPct - b.currentUtilPct,
    },
    {
      title: '优化后利用率',
      dataIndex: 'optimizedUtilPct',
      key: 'optimizedUtilPct',
      width: 130,
      render: (pct: number) => (
        <div>
          <div style={{ fontSize: '12px', marginBottom: '4px' }}>{pct.toFixed(1)}%</div>
          <Progress
            percent={pct}
            size="small"
            showInfo={false}
            strokeColor={getUtilizationColor(pct)}
          />
        </div>
      ),
      sorter: (a, b) => a.optimizedUtilPct - b.optimizedUtilPct,
    },
    {
      title: '已用/目标',
      key: 'capacity',
      width: 150,
      render: (_, record) => (
        <div style={{ fontSize: '12px' }}>
          <div>
            <span style={{ fontWeight: 'bold' }}>{record.usedCapacityT.toFixed(3)}</span> /{' '}
            {record.targetCapacityT.toFixed(3)} 吨
          </div>
        </div>
      ),
    },
    {
      title: '机会空间',
      dataIndex: 'opportunitySpaceT',
      key: 'opportunitySpaceT',
      width: 120,
      render: (space: number) => (
        <span
          style={{
            color: space > 0 ? '#52c41a' : '#ff4d4f',
            fontWeight: 'bold',
          }}
        >
          {space.toFixed(3)}吨
        </span>
      ),
      sorter: (a, b) => a.opportunitySpaceT - b.opportunitySpaceT,
      defaultSortOrder: 'descend',
    },
    {
      title: '描述',
      dataIndex: 'description',
      key: 'description',
      ellipsis: true,
      render: (desc: string) => desc || '-',
    },
    {
      title: '建议操作',
      dataIndex: 'recommendedActions',
      key: 'recommendedActions',
      ellipsis: true,
      render: (actions: string[]) => (
        <div style={{ fontSize: '12px' }}>
          {actions.length > 0 ? actions[0] : '-'}
          {actions.length > 1 && (
            <Tag color="blue" style={{ marginLeft: '4px' }}>
              +{actions.length - 1}
            </Tag>
          )}
        </div>
      ),
    },
  ];

  // ==========================================
  // 加载状态
  // ==========================================

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: embedded ? '40px 0' : '100px 0' }}>
        <Spin size="large" tip="正在加载容量优化机会数据...">
          <div style={{ minHeight: 80 }} />
        </Spin>
      </div>
    );
  }

  // ==========================================
  // 错误状态
  // ==========================================

  if (error) {
    return (
      <Alert
        message="数据加载失败"
        description={error.message || '未知错误'}
        type="error"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  if (!versionId) {
    return (
      <Alert
        message="未选择排产版本"
        description="请先在主界面选择一个排产版本"
        type="warning"
        showIcon
        style={{ margin: embedded ? 0 : '20px' }}
      />
    );
  }

  // ==========================================
  // 主界面
  // ==========================================

  return (
    <div style={{ padding: embedded ? 0 : 24 }}>
      {!embedded ? (
        <div style={{ marginBottom: 24 }}>
          <h2>
            <RiseOutlined style={{ marginRight: 8 }} />
            D6决策：容量优化机会
          </h2>
          <p style={{ color: '#8c8c8c', marginBottom: 16 }}>
            识别未充分利用的产能，提供负载平衡和优化建议，最大化生产效率
          </p>

          <Space>
            <span>查看范围：</span>
            <Select
              value={selectedDays}
              onChange={setSelectedDays}
              style={{ width: 120 }}
              options={[
                { label: '7天', value: 7 },
                { label: '14天', value: 14 },
                { label: '30天', value: 30 },
                { label: '60天', value: 60 },
              ]}
            />
          </Space>
        </div>
      ) : null}

      {/* 统计卡片 */}
      <Row gutter={embedded ? 12 : 16} style={{ marginBottom: embedded ? 12 : 24 }}>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="优化机会总数"
              value={stats.totalOpportunities}
              prefix={<RiseOutlined />}
              valueStyle={{ color: stats.totalOpportunities > 0 ? '#1677ff' : '#8c8c8c' }}
            />
          </Card>
        </Col>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="未充分利用"
              value={stats.underutilizedCount}
              prefix={<CalendarOutlined />}
              valueStyle={{ color: stats.underutilizedCount > 0 ? '#52c41a' : '#8c8c8c' }}
            />
          </Card>
        </Col>
        <Col span={4}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="机会空间总计"
              value={stats.totalOpportunitySpace}
              precision={1}
              suffix="吨"
              prefix={<SwapOutlined />}
              valueStyle={{ color: stats.totalOpportunitySpace > 0 ? '#52c41a' : '#8c8c8c' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="平均当前利用率"
              value={stats.avgCurrentUtilization}
              precision={1}
              suffix="%"
              valueStyle={{
                color: getUtilizationColor(stats.avgCurrentUtilization),
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="平均优化后利用率"
              value={stats.avgOptimizedUtilization}
              precision={1}
              suffix="%"
              valueStyle={{
                color: getUtilizationColor(stats.avgOptimizedUtilization),
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 完整表格 */}
      <Card
        title={`容量优化机会 (共 ${data?.totalCount ?? 0} 条)`}
        size={embedded ? 'small' : undefined}
        style={{ marginBottom: embedded ? 12 : 24 }}
      >
        <Table<CapacityOpportunity>
          columns={columns}
          dataSource={data?.items || []}
          rowKey={(record) => `${record.planDate}-${record.machineCode}`}
          pagination={{
            pageSize: 20,
            showSizeChanger: true,
            showQuickJumper: true,
          }}
          scroll={{ x: 1400 }}
          rowClassName={(record) => {
            if (record.opportunitySpaceT > 100) {
              return 'high-priority-row';
            }
            return '';
          }}
          expandable={
            openWithDrawer
              ? undefined
              : {
                  expandedRowRender: (record) => <OpportunityDetail opportunity={record} />,
                  rowExpandable: (record) => record.recommendedActions.length > 0,
                }
          }
          onRow={
            openWithDrawer
              ? (record) => ({
                  onClick: () =>
                    onOpenDrilldown?.({
                      kind: 'capacityOpportunity',
                      machineCode: record.machineCode,
                      planDate: record.planDate,
                    }),
                  style: { cursor: 'pointer' },
                })
              : undefined
          }
        />
      </Card>

      {/* 自定义样式 */}
      <style>
        {`
          .high-priority-row {
            background-color: ${warningBg};
          }
          .high-priority-row:hover {
            background-color: ${warningBgHover} !important;
          }
        `}
      </style>
    </div>
  );
};

// ==========================================
// 机会详情展开组件
// ==========================================

interface OpportunityDetailProps {
  opportunity: CapacityOpportunity;
}

const OpportunityDetail: React.FC<OpportunityDetailProps> = ({ opportunity }) => {
  const { token } = theme.useToken();
  const opportunityType = parseOpportunityType(opportunity.opportunityType);

  return (
    <div style={{ padding: 16, backgroundColor: token.colorFillQuaternary }}>
      <Row gutter={16}>
        {/* 左侧：建议操作 */}
        <Col span={12}>
          <h4>建议操作：</h4>
          <ul style={{ paddingLeft: '20px', marginBottom: 0 }}>
            {opportunity.recommendedActions.map((action, index) => (
              <li key={index} style={{ marginBottom: '8px', color: '#1677ff' }}>
                {action}
              </li>
            ))}
          </ul>
          {/* 潜在收益 */}
          {opportunity.potentialBenefits.length > 0 && (
            <>
              <h4 style={{ marginTop: '16px' }}>潜在收益：</h4>
              <ul style={{ paddingLeft: '20px', marginBottom: 0 }}>
                {opportunity.potentialBenefits.map((benefit, index) => (
                  <li key={index} style={{ marginBottom: '8px', color: '#52c41a' }}>
                    {benefit}
                  </li>
                ))}
              </ul>
            </>
          )}
        </Col>

        {/* 右侧：机会类型说明 */}
        <Col span={12}>
          <h4>机会类型说明：</h4>
          <Space direction="vertical" style={{ width: '100%' }}>
            <div>
              <Tag color={OPPORTUNITY_TYPE_COLORS[opportunityType]}>
                {OPPORTUNITY_TYPE_LABELS[opportunityType]}
              </Tag>
              <span style={{ fontSize: 12, color: token.colorTextSecondary, marginLeft: 8 }}>
                {getOpportunityTypeDescription(opportunityType)}
              </span>
            </div>
          </Space>

          {/* 描述 */}
          {opportunity.description && (
            <div style={{ marginTop: '16px' }}>
              <h4>详细描述：</h4>
              <p style={{ color: token.colorTextSecondary }}>{opportunity.description}</p>
            </div>
          )}
        </Col>
      </Row>
    </div>
  );
};

// ==========================================
// 辅助函数
// ==========================================

function getOpportunityTypeDescription(type: OpportunityType): string {
  switch (type) {
    case 'UNDERUTILIZED':
      return '容量使用率低于70%，存在较大优化空间';
    case 'MOVABLE_LOAD':
      return '可以将其他日期的材料移动到此日期';
    case 'STRUCTURE_FIX':
      return '可以优化产品结构比例以提高效率';
    case 'URGENT_INSERTION':
      return '可以插入紧急订单而不影响现有排产';
    case 'LOAD_BALANCE':
      return '可以分散其他日期的负载以平衡产能';
    default:
      return '';
  }
}

// ==========================================
// 默认导出（用于React.lazy）
// ==========================================
export default D6CapacityOpportunity;
