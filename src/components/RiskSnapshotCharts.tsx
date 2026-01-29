import React, { useMemo, useRef, useState, useEffect } from 'react';
import {
  Card,
  Row,
  Col,
  Select,
  DatePicker,
  Button,
  Space,
  Table,
  Tag,
  Alert,
  Statistic,
  Tabs,
  message,
  Dropdown,
} from 'antd';
import {
  LineChartOutlined,
  PieChartOutlined,
  BarChartOutlined,
  DownloadOutlined,
  ReloadOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { Dayjs } from 'dayjs';
import { useNavigate } from 'react-router-dom';
import { dashboardApi, planApi } from '../api/tauri';
import { useEvent } from '../api/eventBus';
import { useActiveVersionId } from '../stores/use-global-store';
import { formatNumber } from '../utils/formatters';
import { exportCSV, exportJSON } from '../utils/exportUtils';
import NoActiveVersionGuide from './NoActiveVersionGuide';

const { Option } = Select;
const { RangePicker } = DatePicker;
const { TabPane } = Tabs;

// D1: DaySummaryDto (snake_case)
interface ReasonItem {
  code: string;
  msg: string;
  weight: number;
  affected_count?: number;
}

interface RiskDaySummary {
  plan_date: string;
  risk_score: number;
  risk_level: string;
  capacity_util_pct: number;
  overload_weight_t: number;
  urgent_failure_count: number;
  top_reasons: ReasonItem[];
  involved_machines: string[];
}

interface DecisionDaySummaryResponse {
  items: RiskDaySummary[];
}

interface RiskSnapshotChartsProps {
  onNavigateToPlan?: () => void;
}

interface VersionOption {
  value: string;
  label: string;
}

const RiskSnapshotCharts: React.FC<RiskSnapshotChartsProps> = ({ onNavigateToPlan }) => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [selectedVersion, setSelectedVersion] = useState<string>('');
  const [versionOptions, setVersionOptions] = useState<VersionOption[]>([]);
  const [rawRiskSnapshots, setRawRiskSnapshots] = useState<RiskDaySummary[]>([]);
  const [mostRiskyDate, setMostRiskyDate] = useState<string | null>(null);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs] | null>(null);
  const [activeTab, setActiveTab] = useState<string>('trend');
  const activeVersionId = useActiveVersionId();
  const navigateToPlan = onNavigateToPlan || (() => navigate('/comparison'));
  const prevActiveVersionIdRef = useRef<string | null>(null);

  // 订阅risk_snapshot_updated事件,自动刷新
  useEvent('risk_snapshot_updated', () => {
    if (!activeVersionId) return;
    // risk_snapshot_updated 语义上通常指“当前激活版本”的快照刷新。
    // 若用户正在查看其他版本，避免把视图强制切回 activeVersionId。
    if (!selectedVersion || selectedVersion === activeVersionId) {
      const vid = selectedVersion || activeVersionId;
      loadRiskSnapshots(vid);
      loadMostRiskyDate(vid);
    }
  });

  // 风险等级颜色映射
  const riskLevelColors: Record<string, string> = {
    CRITICAL: 'red',
    HIGH: 'volcano',
    MEDIUM: 'orange',
    LOW: 'green',
  };

  // 加载风险快照数据
  const loadRiskSnapshots = async (versionId: string) => {
    if (!versionId) {
      message.warning('请先激活一个版本');
      return;
    }

    setLoading(true);
    try {
      const result = (await dashboardApi.listRiskSnapshots(versionId)) as DecisionDaySummaryResponse;
      const items = result?.items || [];
      setRawRiskSnapshots(items);
      message.success(`成功加载 ${items.length} 条风险摘要`);
    } catch (error: any) {
      console.error('加载风险快照失败:', error);
    } finally {
      setLoading(false);
    }
  };

  // 加载最危险日期
  const loadMostRiskyDate = async (versionId: string) => {
    if (!versionId) return;
    try {
      const result = (await dashboardApi.getMostRiskyDate(versionId)) as DecisionDaySummaryResponse;
      const most = result?.items?.[0];
      setMostRiskyDate(most?.plan_date || null);
    } catch (error: any) {
      console.error('加载最危险日期失败:', error);
    }
  };

  // 风险详情表格列定义
  const columns: ColumnsType<RiskDaySummary> = [
    {
      title: '日期',
      dataIndex: 'plan_date',
      key: 'plan_date',
      width: 120,
      render: (date: string) => {
        const isMostRisky = date === mostRiskyDate;
        return (
          <Space>
            {date}
            {isMostRisky && (
              <Tag color="red" icon={<WarningOutlined />}>
                最危险
              </Tag>
            )}
          </Space>
        );
      },
    },
    {
      title: '风险分数',
      dataIndex: 'risk_score',
      key: 'risk_score',
      width: 100,
      sorter: (a, b) => a.risk_score - b.risk_score,
      render: (score: number) => (
        <span style={{ fontWeight: 'bold', color: score > 60 ? '#cf1322' : '#52c41a' }}>
          {score}
        </span>
      ),
    },
    {
      title: '风险等级',
      dataIndex: 'risk_level',
      key: 'risk_level',
      width: 100,
      render: (level: string) => (
        <Tag color={riskLevelColors[level] || 'default'}>{level}</Tag>
      ),
    },
    {
      title: '产能利用率(%)',
      dataIndex: 'capacity_util_pct',
      key: 'capacity_util_pct',
      width: 100,
      render: (val: number) => formatNumber(val, 1),
    },
    {
      title: '超载吨数(t)',
      dataIndex: 'overload_weight_t',
      key: 'overload_weight_t',
      width: 100,
      render: (val: number) => formatNumber(val, 1),
    },
    {
      title: '紧急单失败数',
      dataIndex: 'urgent_failure_count',
      key: 'urgent_failure_count',
      width: 100,
    },
    {
      title: '涉及机组',
      dataIndex: 'involved_machines',
      key: 'involved_machines',
      width: 100,
      render: (machines: string[]) => (machines && machines.length > 0 ? machines.join(',') : '-'),
    },
  ];

  // 默认跟随“当前激活版本”；如果用户手动切换到其他版本，则保持其选择不被覆盖。
  useEffect(() => {
    const prev = prevActiveVersionIdRef.current;
    prevActiveVersionIdRef.current = activeVersionId;

    if (!activeVersionId) return;
    if (!selectedVersion || selectedVersion === prev) {
      setSelectedVersion(activeVersionId);
    }
  }, [activeVersionId, selectedVersion]);

  // 版本切换时自动刷新（避免用户必须手动点击“刷新”）
  useEffect(() => {
    if (selectedVersion) {
      loadRiskSnapshots(selectedVersion);
      loadMostRiskyDate(selectedVersion);
    }
  }, [selectedVersion]);

  const riskSnapshots = useMemo(() => {
    const base = Array.isArray(rawRiskSnapshots) ? rawRiskSnapshots : [];
    const filtered = dateRange
      ? base.filter((s: RiskDaySummary) => {
          const d = dayjs(s.plan_date);
          return (
            d.isAfter(dateRange[0].startOf('day').subtract(1, 'millisecond')) &&
            d.isBefore(dateRange[1].endOf('day').add(1, 'millisecond'))
          );
        })
      : base;

    // 确保按日期升序，避免不同后端排序导致图表抖动
    return [...filtered].sort((a, b) => a.plan_date.localeCompare(b.plan_date));
  }, [rawRiskSnapshots, dateRange]);

  // 加载版本下拉选项（跨方案汇总）
  useEffect(() => {
    if (!activeVersionId) return;

    (async () => {
      try {
        const plans = await planApi.listPlans();
        const options: VersionOption[] = [];

        for (const plan of plans || []) {
          const versions = await planApi.listVersions(plan.plan_id);
          for (const v of versions || []) {
            options.push({
              value: v.version_id,
              label: `${plan.plan_name} / V${v.version_no} (${v.status})`,
            });
          }
        }

        setVersionOptions(options);
      } catch (error: any) {
        console.error('加载版本列表失败:', error);
        setVersionOptions([]);
      }
    })();
  }, [activeVersionId]);

  // 没有激活版本时显示引导
  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="风险分析需要一个激活的排产版本作为基础"
        onNavigateToPlan={navigateToPlan}
      />
    );
  }

  // 渲染风险趋势图(使用简化的CSS实现)
  const renderTrendChart = () => {
    const maxScore = Math.max(...riskSnapshots.map((s) => s.risk_score), 0);

    return (
      <div style={{ padding: '20px' }}>
        <div style={{ display: 'flex', alignItems: 'flex-end', height: '300px', gap: '10px' }}>
          {riskSnapshots.map((snapshot, index) => {
            const height = maxScore > 0 ? (snapshot.risk_score / maxScore) * 100 : 0;
            const color =
              snapshot.risk_score > 60
                ? '#cf1322'
                : snapshot.risk_score > 40
                ? '#fa8c16'
                : '#52c41a';

            return (
              <div
                key={index}
                style={{
                  flex: 1,
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  gap: '8px',
                }}
              >
                <div
                  style={{
                    fontSize: '12px',
                    fontWeight: 'bold',
                    color: color,
                  }}
                >
                  {snapshot.risk_score}
                </div>
                <div
                  style={{
                    width: '100%',
                    height: `${height}%`,
                    backgroundColor: color,
                    borderRadius: '4px 4px 0 0',
                    transition: 'all 0.3s',
                    cursor: 'pointer',
                  }}
                  title={`${snapshot.plan_date}: ${snapshot.risk_score}`}
                />
                <div
                  style={{
                    fontSize: '11px',
                    color: '#8c8c8c',
                    transform: 'rotate(-45deg)',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {dayjs(snapshot.plan_date).format('MM-DD')}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  // 渲染风险分布图(使用简化的CSS实现)
  const renderDistributionChart = () => {
    const distribution: Record<string, number> = {};
    riskSnapshots.forEach((snapshot) => {
      distribution[snapshot.risk_level] = (distribution[snapshot.risk_level] || 0) + 1;
    });

    const total = riskSnapshots.length;
    const data = Object.entries(distribution).map(([level, count]) => ({
      level,
      count,
      percentage: formatNumber((count / total) * 100, 1),
      color: riskLevelColors[level] || '#d9d9d9',
    }));

    return (
      <div style={{ padding: '20px' }}>
        <Row gutter={16}>
          {data.map((item, index) => (
            <Col span={6} key={index}>
              <Card
                style={{
                  textAlign: 'center',
                  borderColor: item.color,
                  borderWidth: 2,
                }}
              >
                <Statistic
                  title={item.level}
                  value={item.count}
                  suffix={`天 (${item.percentage}%)`}
                  valueStyle={{ color: item.color }}
                />
              </Card>
            </Col>
          ))}
        </Row>
      </div>
    );
  };

  // 渲染风险指标卡片
  const renderRiskMetrics = () => {
    if (riskSnapshots.length === 0) return null;

    const avgRiskScore =
      riskSnapshots.reduce((sum, s) => sum + s.risk_score, 0) / riskSnapshots.length;
    const totalUrgentFailures = riskSnapshots.reduce((sum, s) => sum + (s.urgent_failure_count || 0), 0);
    const totalOverloadT = riskSnapshots.reduce((sum, s) => sum + (s.overload_weight_t || 0), 0);

    // 获取最危险日期的风险分数
    const mostRiskySnapshot = riskSnapshots.find((s) => s.plan_date === mostRiskyDate);
    const maxRiskScore = mostRiskySnapshot?.risk_score || 0;

    return (
      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={6}>
          <Card>
            <Statistic
              title="平均风险分数"
              value={formatNumber(avgRiskScore, 1)}
              valueStyle={{ color: avgRiskScore > 50 ? '#cf1322' : '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card style={{ borderColor: mostRiskyDate ? '#ff4d4f' : undefined }}>
            <Statistic
              title="最高风险分数"
              value={maxRiskScore}
              suffix={mostRiskyDate ? `(${mostRiskyDate})` : ''}
              valueStyle={{ color: '#cf1322' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="累计紧急单失败"
              value={totalUrgentFailures}
              suffix="单"
              valueStyle={{ color: totalUrgentFailures > 0 ? '#cf1322' : '#52c41a' }}
            />
          </Card>
        </Col>
        <Col span={6}>
          <Card>
            <Statistic
              title="累计超载吨数"
              value={formatNumber(totalOverloadT, 1)}
              suffix="t"
              valueStyle={{ color: '#fa8c16' }}
            />
          </Card>
        </Col>
      </Row>
    );
  };

  return (
    <div style={{ padding: '24px' }}>
      {/* 标题和操作栏 */}
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>风险快照分析</h2>
        </Col>
        <Col>
          <Space>
            <Select
              style={{ width: 200 }}
              placeholder="选择版本"
              value={selectedVersion}
              onChange={setSelectedVersion}
            >
              {versionOptions.map((opt) => (
                <Option key={opt.value} value={opt.value}>
                  {opt.label}
                </Option>
              ))}
            </Select>
            <RangePicker
              placeholder={['开始日期', '结束日期']}
              value={dateRange as any}
              onChange={(dates) => {
                if (dates && dates[0] && dates[1]) {
                  setDateRange([dates[0], dates[1]]);
                } else {
                  setDateRange(null);
                }
              }}
              format="YYYY-MM-DD"
            />
            <Button
              icon={<ReloadOutlined />}
              onClick={() => {
                loadRiskSnapshots(selectedVersion);
                loadMostRiskyDate(selectedVersion);
              }}
            >
              刷新
            </Button>
            <Dropdown
              menu={{
                items: [
                  {
                    label: '导出为 CSV',
                    key: 'csv',
                    onClick: () => {
                      try {
                        const data = riskSnapshots.map((snapshot) => ({
                          日期: snapshot.plan_date,
                          风险分数: formatNumber(snapshot.risk_score, 1),
                          风险等级: snapshot.risk_level,
                          产能利用率: formatNumber(snapshot.capacity_util_pct, 1),
                          超载吨数: formatNumber(snapshot.overload_weight_t, 1),
                          紧急失败数: snapshot.urgent_failure_count,
                          涉及机组: (snapshot.involved_machines || []).join(','),
                        }));
                        exportCSV(data, '风险摘要(D1)');
                        message.success('导出成功');
                      } catch (error: any) {
                        message.error(`导出失败: ${error.message}`);
                      }
                    },
                  },
                  {
                    label: '导出为 JSON',
                    key: 'json',
                    onClick: () => {
                      try {
                        exportJSON(riskSnapshots, '风险快照');
                        message.success('导出成功');
                      } catch (error: any) {
                        message.error(`导出失败: ${error.message}`);
                      }
                    },
                  },
                ],
              }}
            >
              <Button icon={<DownloadOutlined />}>导出</Button>
            </Dropdown>
          </Space>
        </Col>
      </Row>

      {/* 最危险日期提醒 */}
      {mostRiskyDate && (
        <Alert
          message="风险预警"
          description={`最危险日期: ${mostRiskyDate}，请重点关注该日期的排产情况`}
          type="warning"
          showIcon
          icon={<WarningOutlined />}
          style={{ marginBottom: 16 }}
          action={
            <Button
              size="small"
              type="primary"
              danger
              onClick={() => setActiveTab('details')}
            >
              查看详情
            </Button>
          }
        />
      )}

      {/* 风险指标卡片 */}
      {renderRiskMetrics()}

      {/* 图表标签页 */}
      <Card>
        <Tabs activeKey={activeTab} onChange={setActiveTab}>
          <TabPane
            tab={
              <span>
                <LineChartOutlined />
                风险趋势
              </span>
            }
            key="trend"
          >
            <div>
              <h3>风险分数趋势图</h3>
              <p style={{ color: '#8c8c8c' }}>
                显示各日期的风险分数变化趋势，分数越高风险越大
              </p>
              {renderTrendChart()}
            </div>
          </TabPane>

          <TabPane
            tab={
              <span>
                <PieChartOutlined />
                风险分布
              </span>
            }
            key="distribution"
          >
            <div>
              <h3>风险等级分布</h3>
              <p style={{ color: '#8c8c8c' }}>
                显示不同风险等级的天数分布情况
              </p>
              {renderDistributionChart()}
            </div>
          </TabPane>

          <TabPane
            tab={
              <span>
                <BarChartOutlined />
                详细数据
              </span>
            }
            key="details"
          >
            <div>
              <h3>风险快照详情</h3>
              <p style={{ color: '#8c8c8c' }}>
                显示每日的详细风险指标数据
              </p>
              <Table
                columns={columns}
                dataSource={riskSnapshots}
                loading={loading}
                pagination={false}
                rowKey="plan_date"
                size="small"
                rowClassName={(record) =>
                  record.plan_date === mostRiskyDate ? 'most-risky-row' : ''
                }
              />
              <style>{`
                .most-risky-row {
                  background-color: #fff1f0 !important;
                  font-weight: bold;
                }
                .most-risky-row:hover {
                  background-color: #ffccc7 !important;
                }
              `}</style>
            </div>
          </TabPane>
        </Tabs>
      </Card>
    </div>
  );
};

export default RiskSnapshotCharts;
