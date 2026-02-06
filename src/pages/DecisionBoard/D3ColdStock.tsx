// ==========================================
// D3决策：冷料库龄分析页面
// ==========================================
// 职责: 展示冷料库龄分布和压库风险，支持按机组查看结构缺口
// ==========================================

import React, { useEffect, useMemo, useState } from 'react';
import { Card, Row, Col, Statistic, Tag, Spin, Alert, Space, Select, Descriptions, Table } from 'antd';
import {
  WarningOutlined,
  DatabaseOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { useSearchParams } from 'react-router-dom';
import { useColdStockProfile } from '../../hooks/queries/use-decision-queries';
import type { DrilldownSpec } from '../../hooks/useRiskOverviewData';
import { useActiveVersionId } from '../../stores/use-global-store';
import { ColdStockChart } from '../../components/charts/ColdStockChart';
import { EmptyState } from '../../components/EmptyState';
import type { ColdStockBucket, AgeBin, PressureLevel, ReasonItem } from '../../types/decision';
import { formatNumber, formatWeight } from '../../utils/formatters';

// ==========================================
// 压力等级颜色映射
// ==========================================
const PRESSURE_LEVEL_COLORS: Record<PressureLevel, string> = {
  LOW: '#52c41a',
  MEDIUM: '#1677ff',
  HIGH: '#faad14',
  CRITICAL: '#ff4d4f',
};

const PRESSURE_LEVEL_LABELS: Record<PressureLevel, string> = {
  LOW: '低压',
  MEDIUM: '中压',
  HIGH: '高压',
  CRITICAL: '严重',
};

// ==========================================
// 库龄区间标签
// ==========================================
const AGE_BIN_LABELS: Record<AgeBin, string> = {
  '0-7': '0-7天',
  '8-14': '8-14天',
  '15-30': '15-30天',
  '30+': '30天以上',
};

const AGE_BIN_COLORS: Record<AgeBin, string> = {
  '0-7': '#52c41a',
  '8-14': '#1677ff',
  '15-30': '#faad14',
  '30+': '#ff4d4f',
};

// ==========================================
// 结构缺口（兼容旧枚举值 + 当前后端的描述性文本）
// ==========================================
function hasStructureGap(gap: string | null | undefined): boolean {
  if (!gap) return false;
  const trimmed = gap.trim();
  return trimmed !== '' && trimmed !== 'NONE' && trimmed !== '无';
}

function formatStructureGap(gap: string): string {
  const trimmed = (gap || '').trim();
  const labels: Record<string, string> = {
    SIZE_MISMATCH: '尺寸不匹配',
    GRADE_CONFLICT: '钢种冲突',
    NO_CAMPAIGN: '无换辊计划',
    CAPACITY_FULL: '产能满载',
    NONE: '无缺口',
    无: '无缺口',
  };
  return labels[trimmed] ?? trimmed;
}

// ==========================================
// 主组件
// ==========================================

interface D3ColdStockProps {
  embedded?: boolean;
  onOpenDrilldown?: (spec: DrilldownSpec) => void;
}

export const D3ColdStock: React.FC<D3ColdStockProps> = ({ embedded, onOpenDrilldown }) => {
  const versionId = useActiveVersionId();
  const [searchParams] = useSearchParams();
  const [selectedMachine, setSelectedMachine] = useState<string | null>(null);
  const [selectedAgeBin, setSelectedAgeBin] = useState<AgeBin | null>(null);
  const [selectedPressureLevel, setSelectedPressureLevel] = useState<PressureLevel | null>(null);

  // 获取冷料数据（不默认筛掉低/中压，避免驾驶舱 drill-down 后“页面为空”）
  const { data, isLoading, error } = useColdStockProfile(
    { versionId: versionId || '' },
    { enabled: !!versionId }
  );

  const handleSelectMachine = (machine: string | null) => {
    if (embedded && onOpenDrilldown) {
      setSelectedMachine(machine);
      onOpenDrilldown({ kind: 'coldStock', machineCode: machine || undefined });
      return;
    }
    setSelectedMachine(machine);
    // 切换机组时清空桶定位，避免“定位到不存在的桶”
    setSelectedAgeBin(null);
    setSelectedPressureLevel(null);
  };

  // 支持 Dashboard drill-down：
  // /decision/d3-cold-stock?machine=H031&ageBin=0-7&pressureLevel=HIGH
  useEffect(() => {
    const machine = searchParams.get('machine');
    const ageBin = searchParams.get('ageBin');
    const pressureLevelRaw = searchParams.get('pressureLevel');

    if (machine) {
      setSelectedMachine(machine);
    }

    if (ageBin && (['0-7', '8-14', '15-30', '30+'] as string[]).includes(ageBin)) {
      setSelectedAgeBin(ageBin as AgeBin);
    }

    if (pressureLevelRaw) {
      const normalized = pressureLevelRaw.toUpperCase();
      if ((['LOW', 'MEDIUM', 'HIGH', 'CRITICAL'] as string[]).includes(normalized)) {
        setSelectedPressureLevel(normalized as PressureLevel);
      }
    }
  }, [searchParams]);

  // M1修复：优化按机组分组统计，单次遍历计算所有指标
  const machineStats = useMemo(() => {
    if (!data?.items || data.items.length === 0) return [];

    // 压力等级排序映射
    const levelOrder: Record<PressureLevel, number> = {
      LOW: 0,
      MEDIUM: 1,
      HIGH: 2,
      CRITICAL: 3,
    };

    // 使用Map存储每个机组的统计信息（避免多次遍历）
    const machineMap = new Map<
      string,
      {
        buckets: ColdStockBucket[];
        totalCount: number;
        totalWeight: number;
        maxPressureScore: number;
        weightedAgeSum: number; // 用于计算加权平均库龄
        maxAge: number;
        highestPressureLevel: PressureLevel;
      }
    >();

    // M1修复：单次遍历完成所有统计计算
    data.items.forEach((bucket) => {
      const machine = bucket.machineCode;
      const existing = machineMap.get(machine);

      if (existing) {
        // 更新现有机组的统计
        existing.buckets.push(bucket);
        existing.totalCount += bucket.count;
        existing.totalWeight += bucket.weightT;
        existing.maxPressureScore = Math.max(existing.maxPressureScore, bucket.pressureScore);
        existing.weightedAgeSum += bucket.avgAgeDays * bucket.count;
        existing.maxAge = Math.max(existing.maxAge, bucket.maxAgeDays);

        // 更新最高压力等级
        if (levelOrder[bucket.pressureLevel] > levelOrder[existing.highestPressureLevel]) {
          existing.highestPressureLevel = bucket.pressureLevel;
        }
      } else {
        // 新机组初始化
        machineMap.set(machine, {
          buckets: [bucket],
          totalCount: bucket.count,
          totalWeight: bucket.weightT,
          maxPressureScore: bucket.pressureScore,
          weightedAgeSum: bucket.avgAgeDays * bucket.count,
          maxAge: bucket.maxAgeDays,
          highestPressureLevel: bucket.pressureLevel,
        });
      }
    });

    // 转换为最终的统计数组
    return Array.from(machineMap.entries())
      .map(([machine, stats]) => ({
        machine,
        buckets: stats.buckets,
        totalCount: stats.totalCount,
        totalWeight: stats.totalWeight,
        maxPressureScore: stats.maxPressureScore,
        avgAge: stats.totalCount > 0 ? stats.weightedAgeSum / stats.totalCount : 0,
        maxAge: stats.maxAge,
        highestPressureLevel: stats.highestPressureLevel,
      }))
      .sort((a, b) => b.maxPressureScore - a.maxPressureScore);
  }, [data]);

  // 全局统计
  const globalStats = useMemo(() => {
    if (!data?.items || data.items.length === 0) {
      return {
        totalMachines: 0,
        totalColdStock: 0,
        totalWeight: 0,
        highPressureCount: 0,
      };
    }

    const uniqueMachines = new Set(data.items.map((b) => b.machineCode));
    const totalColdStock = data.items.reduce((sum, b) => sum + b.count, 0);
    const totalWeight = data.items.reduce((sum, b) => sum + b.weightT, 0);
    const highPressureCount = data.items.filter(
      (b) => b.pressureLevel === 'HIGH' || b.pressureLevel === 'CRITICAL'
    ).reduce((sum, b) => sum + b.count, 0);

    return {
      totalMachines: uniqueMachines.size,
      totalColdStock,
      totalWeight,
      highPressureCount,
    };
  }, [data]);

  // 选中机组的数据
  const selectedMachineData = useMemo(() => {
    if (!selectedMachine) return null;
    return machineStats.find((m) => m.machine === selectedMachine) || null;
  }, [selectedMachine, machineStats]);

  const selectedBucket = useMemo(() => {
    if (!selectedMachineData) return null;
    if (!selectedAgeBin && !selectedPressureLevel) return null;

    return (
      selectedMachineData.buckets.find((b) => {
        if (selectedAgeBin && b.ageBin !== selectedAgeBin) return false;
        if (selectedPressureLevel && b.pressureLevel !== selectedPressureLevel) return false;
        return true;
      }) || null
    );
  }, [selectedMachineData, selectedAgeBin, selectedPressureLevel]);

  // ==========================================
  // 加载状态
  // ==========================================

  if (isLoading) {
    return (
      <div style={{ textAlign: 'center', padding: embedded ? '40px 0' : '100px 0' }}>
        <Spin size="large" tip="正在加载冷料库龄数据...">
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
            <DatabaseOutlined style={{ marginRight: 8 }} />
            D3决策：冷料库龄分析
          </h2>
          <p style={{ color: '#8c8c8c', marginBottom: 16 }}>
            展示各机组冷料库龄分布，识别压库风险和结构缺口
          </p>

          <Space>
            <span>选择机组：</span>
            <Select
              value={selectedMachine}
              onChange={handleSelectMachine}
              style={{ width: 200 }}
              placeholder="查看全部机组"
              allowClear
              options={machineStats.map((m) => ({
                label: `${m.machine} (${m.totalCount}个材料)`,
                value: m.machine,
              }))}
            />
          </Space>
        </div>
      ) : (
        <div style={{ marginBottom: 12 }}>
          <Space align="center" wrap>
            <span>机组：</span>
            <Select
              size="small"
              value={selectedMachine}
              onChange={handleSelectMachine}
              style={{ width: 200 }}
              placeholder="全部"
              allowClear
              options={machineStats.map((m) => ({
                label: `${m.machine} (${m.totalCount}个)`,
                value: m.machine,
              }))}
            />
          </Space>
        </div>
      )}

      {/* 统计卡片 */}
      <Row gutter={embedded ? 12 : 16} style={{ marginBottom: embedded ? 12 : 24 }}>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="涉及机组数"
              value={globalStats.totalMachines}
              prefix={<DatabaseOutlined />}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="冷料总数"
              value={globalStats.totalColdStock}
              suffix="个"
              valueStyle={{
                color: globalStats.totalColdStock > 100 ? '#faad14' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="冷料总重量"
              value={formatNumber(globalStats.totalWeight, 3)}
              suffix="吨"
              valueStyle={{
                color: globalStats.totalWeight > 1000 ? '#faad14' : '#52c41a',
              }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card size={embedded ? 'small' : undefined}>
            <Statistic
              title="高压库存数"
              value={globalStats.highPressureCount}
              prefix={<WarningOutlined />}
              valueStyle={{
                color: globalStats.highPressureCount > 0 ? '#ff4d4f' : '#52c41a',
              }}
            />
          </Card>
        </Col>
      </Row>

      {/* 库龄分布图表 */}
      <Card
        title="库龄分布图（按机组）"
        style={{ marginBottom: '24px' }}
        extra={
          <Space>
            <InfoCircleOutlined />
            <span style={{ fontSize: '12px', color: '#8c8c8c' }}>
              点击柱状图查看机组详情
            </span>
          </Space>
        }
      >
        {data && data.items.length > 0 ? (
          <ColdStockChart
            data={data.items}
            onMachineClick={handleSelectMachine}
            selectedMachine={selectedMachine}
          />
        ) : (
          <EmptyState type="data" style={{ padding: '40px 0' }} />
        )}
      </Card>

      {/* 机组统计表 */}
      <Card title="机组冷料统计" style={{ marginBottom: '24px' }}>
        <MachineStatsTable data={machineStats} onRowClick={handleSelectMachine} />
      </Card>

      {/* 选中机组的详细信息 */}
      {!embedded && selectedMachineData && (
        <Card
          title={`${selectedMachineData.machine} 冷料详情`}
          extra={
            <Tag color={PRESSURE_LEVEL_COLORS[selectedMachineData.highestPressureLevel]}>
              {PRESSURE_LEVEL_LABELS[selectedMachineData.highestPressureLevel]}
            </Tag>
          }
        >
          <Descriptions column={4} bordered size="small" style={{ marginBottom: '16px' }}>
            <Descriptions.Item label="冷料总数">
              {selectedMachineData.totalCount}个
            </Descriptions.Item>
            <Descriptions.Item label="冷料总重量">
              {formatWeight(selectedMachineData.totalWeight)}
            </Descriptions.Item>
            <Descriptions.Item label="平均库龄">
              {formatNumber(selectedMachineData.avgAge, 2)}天
            </Descriptions.Item>
            <Descriptions.Item label="最大库龄">
              {selectedMachineData.maxAge}天
            </Descriptions.Item>
          </Descriptions>

          {/* 库龄分布详情 */}
          <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>库龄分布</h4>
          <Row gutter={16} style={{ marginBottom: '16px' }}>
            {selectedMachineData.buckets.map((bucket) => (
              <Col key={bucket.ageBin} span={6}>
                <Card
                  size="small"
                  style={{
                    borderLeft: `4px solid ${AGE_BIN_COLORS[bucket.ageBin]}`,
                    cursor: 'pointer',
                    boxShadow:
                      selectedBucket?.ageBin === bucket.ageBin ? '0 0 0 2px rgba(22, 119, 255, 0.35) inset' : undefined,
                    background:
                      selectedBucket?.ageBin === bucket.ageBin ? 'rgba(22, 119, 255, 0.06)' : undefined,
                  }}
                  onClick={() => {
                    if (embedded && onOpenDrilldown) {
                      setSelectedAgeBin(bucket.ageBin);
                      setSelectedPressureLevel(bucket.pressureLevel);
                      onOpenDrilldown({
                        kind: 'coldStock',
                        machineCode: selectedMachineData.machine,
                        ageBin: bucket.ageBin,
                        pressureLevel: bucket.pressureLevel,
                      });
                      return;
                    }
                    setSelectedAgeBin(bucket.ageBin);
                    setSelectedPressureLevel(bucket.pressureLevel);
                  }}
                >
                  <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>
                    {AGE_BIN_LABELS[bucket.ageBin]}
                  </div>
                  <div>数量: {bucket.count}个</div>
                  <div>重量: {formatWeight(bucket.weightT)}</div>
                  <div>
                    压力:
                    <Tag
                      color={PRESSURE_LEVEL_COLORS[bucket.pressureLevel]}
                      style={{ marginLeft: '4px' }}
                    >
                      {formatNumber(bucket.pressureScore, 2)}
                    </Tag>
                  </div>
                </Card>
              </Col>
            ))}
          </Row>

          {/* 结构缺口 */}
          {(selectedBucket ? [selectedBucket] : selectedMachineData.buckets).some((b) =>
            hasStructureGap(b.structureGap)
          ) && (
            <>
              <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>结构缺口</h4>
              <Space direction="vertical" style={{ width: '100%' }}>
                {(selectedBucket ? [selectedBucket] : selectedMachineData.buckets)
                  .filter((b) => hasStructureGap(b.structureGap))
                  .map((bucket, index) => (
                    <Card key={index} size="small" type="inner">
                      <Space wrap>
                        <Tag color={AGE_BIN_COLORS[bucket.ageBin]}>
                          {AGE_BIN_LABELS[bucket.ageBin]}
                        </Tag>
                        <Tag color="red">结构缺口</Tag>
                        <span style={{ color: '#595959' }}>
                          {formatStructureGap(bucket.structureGap)}
                        </span>
                      </Space>
                    </Card>
                  ))}
              </Space>
            </>
          )}

          {/* 压库原因 */}
          {(selectedBucket ? [selectedBucket] : selectedMachineData.buckets).some(
            (b) => b.reasons.length > 0
          ) && (
            <>
              <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>压库原因</h4>
              <ReasonsTable
                reasons={(selectedBucket ? [selectedBucket] : selectedMachineData.buckets).flatMap(
                  (b) => b.reasons
                )}
              />
            </>
          )}

          {/* 趋势信息 */}
          {(selectedBucket ? [selectedBucket] : selectedMachineData.buckets).some((b) => b.trend) && (
            <>
              <h4 style={{ marginTop: '16px', marginBottom: '8px' }}>趋势分析</h4>
              <Space direction="vertical" style={{ width: '100%' }}>
                {(selectedBucket ? [selectedBucket] : selectedMachineData.buckets)
                  .filter((b) => b.trend)
                  .map((bucket, index) => (
                    <Card key={index} size="small" type="inner">
                      <Space>
                        <Tag color={AGE_BIN_COLORS[bucket.ageBin]}>
                          {AGE_BIN_LABELS[bucket.ageBin]}
                        </Tag>
                        <span>
                          {bucket.trend!.direction === 'RISING' && '↑ 上升'}
                          {bucket.trend!.direction === 'STABLE' && '→ 稳定'}
                          {bucket.trend!.direction === 'FALLING' && '↓ 下降'}
                        </span>
                        <span style={{ color: '#8c8c8c' }}>
                          变化率: {formatNumber(bucket.trend!.changeRatePct, 2)}%
                        </span>
                      </Space>
                    </Card>
                  ))}
              </Space>
            </>
          )}
        </Card>
      )}
    </div>
  );
};

// ==========================================
// 机组统计表格
// ==========================================

interface MachineStatsTableProps {
  data: Array<{
    machine: string;
    totalCount: number;
    totalWeight: number;
    maxPressureScore: number;
    avgAge: number;
    maxAge: number;
    highestPressureLevel: PressureLevel;
  }>;
  onRowClick: (machine: string) => void;
}

const MachineStatsTable: React.FC<MachineStatsTableProps> = ({ data, onRowClick }) => {
  const columns: ColumnsType<typeof data[0]> = [
    {
      title: '机组',
      dataIndex: 'machine',
      key: 'machine',
      width: 120,
      render: (machine: string) => <Tag color="blue">{machine}</Tag>,
    },
    {
      title: '冷料数量',
      dataIndex: 'totalCount',
      key: 'totalCount',
      width: 100,
      sorter: (a, b) => a.totalCount - b.totalCount,
    },
    {
      title: '冷料重量(吨)',
      dataIndex: 'totalWeight',
      key: 'totalWeight',
      width: 120,
      render: (weight: number) => formatWeight(weight),
      sorter: (a, b) => a.totalWeight - b.totalWeight,
    },
    {
      title: '平均库龄(天)',
      dataIndex: 'avgAge',
      key: 'avgAge',
      width: 120,
      render: (age: number) => formatNumber(age, 2),
      sorter: (a, b) => a.avgAge - b.avgAge,
    },
    {
      title: '最大库龄(天)',
      dataIndex: 'maxAge',
      key: 'maxAge',
      width: 120,
      sorter: (a, b) => a.maxAge - b.maxAge,
    },
    {
      title: '压力等级',
      dataIndex: 'highestPressureLevel',
      key: 'highestPressureLevel',
      width: 100,
      render: (level: PressureLevel) => (
        <Tag color={PRESSURE_LEVEL_COLORS[level]}>
          {PRESSURE_LEVEL_LABELS[level]}
        </Tag>
      ),
      sorter: (a, b) => {
        const order: Record<PressureLevel, number> = {
          LOW: 0,
          MEDIUM: 1,
          HIGH: 2,
          CRITICAL: 3,
        };
        return order[a.highestPressureLevel] - order[b.highestPressureLevel];
      },
      defaultSortOrder: 'descend',
    },
    {
      title: '压力分数',
      dataIndex: 'maxPressureScore',
      key: 'maxPressureScore',
      width: 100,
      render: (score: number) => formatNumber(score, 2),
      sorter: (a, b) => a.maxPressureScore - b.maxPressureScore,
    },
  ];

  return (
    <Table
      columns={columns}
      dataSource={data}
      rowKey="machine"
      size="small"
      pagination={false}
      onRow={(record) => ({
        onClick: () => onRowClick(record.machine),
        style: { cursor: 'pointer' },
      })}
    />
  );
};

// ==========================================
// 原因表格组件
// ==========================================

interface ReasonsTableProps {
  reasons: ReasonItem[];
}

/**
 * 原因代码中文翻译映射
 */
const REASON_CODE_LABELS: Record<string, string> = {
  CAPACITY_UTILIZATION: '产能利用率',
  LOW_REMAINING_CAPACITY: '剩余产能不足',
  HIGH_CAPACITY_PRESSURE: '产能压力高',
  STRUCTURE_GAP: '结构性缺口',
  COLD_STOCK_AGING: '冷料库龄',
  ROLL_CHANGE_CONFLICT: '换辊冲突',
  URGENCY_BACKLOG: '紧急订单积压',
  MATURITY_CONSTRAINT: '适温约束',
  OVERLOAD_RISK: '超载风险',
  SCHEDULING_CONFLICT: '排产冲突',
};

/**
 * 获取原因代码的中文标签
 */
function getReasonCodeLabel(code: string): string {
  return REASON_CODE_LABELS[code] || '其他原因';
}

const ReasonsTable: React.FC<ReasonsTableProps> = ({ reasons }) => {
  // 去重并合并相同code的原因
  const uniqueReasons = useMemo(() => {
    const map = new Map<string, ReasonItem>();
    reasons.forEach((reason) => {
      const existing = map.get(reason.code);
      if (existing) {
        // 合并权重和影响数
        existing.weight = Math.max(existing.weight, reason.weight);
        if (existing.affectedCount && reason.affectedCount) {
          existing.affectedCount += reason.affectedCount;
        }
      } else {
        map.set(reason.code, { ...reason });
      }
    });
    return Array.from(map.values());
  }, [reasons]);

  const columns: ColumnsType<ReasonItem> = [
    {
      title: '代码',
      dataIndex: 'code',
      key: 'code',
      width: 140,
      render: (code: string) => (
        <Tag color="blue" style={{ maxWidth: '130px', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {getReasonCodeLabel(code)}
        </Tag>
      ),
    },
    {
      title: '原因',
      dataIndex: 'msg',
      key: 'msg',
      ellipsis: { showTitle: true },
      width: 320,
    },
    {
      title: '权重',
      dataIndex: 'weight',
      key: 'weight',
      width: 90,
      render: (weight: number) => <span>{formatNumber(weight * 100, 2)}%</span>,
      sorter: (a, b) => a.weight - b.weight,
      defaultSortOrder: 'descend',
    },
    {
      title: '影响数',
      dataIndex: 'affectedCount',
      key: 'affectedCount',
      width: 90,
      render: (count?: number) => count || '-',
    },
  ];

  return (
    <Table
      columns={columns}
      dataSource={uniqueReasons}
      rowKey="code"
      size="small"
      pagination={false}
    />
  );
};

// ==========================================
// 默认导出（用于React.lazy）
// ==========================================
export default D3ColdStock;
