import React, { useMemo, useState, useEffect } from 'react';
import {
  Card,
  Table,
  Tag,
  Space,
  Button,
  Select,
  DatePicker,
  Input,
  Tooltip,
  Descriptions,
  Modal,
  message,
  Row,
  Col,
  Statistic,
  Dropdown,
} from 'antd';
import {
  FilterOutlined,
  ReloadOutlined,
  DownloadOutlined,
  DragOutlined,
  LockOutlined,
  HolderOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { Dayjs } from 'dayjs';
import { useNavigate } from 'react-router-dom';
import { planApi, materialApi } from '../api/tauri';
import { useEvent } from '../api/eventBus';
import { useActiveVersionId, useCurrentUser } from '../stores/use-global-store';
import { formatWeight, formatDate } from '../utils/formatters';
import { tableFilterEmptyConfig } from './CustomEmpty';
import { exportCSV, exportJSON } from '../utils/exportUtils';
import NoActiveVersionGuide from './NoActiveVersionGuide';
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

const { Option } = Select;
const { Search } = Input;
const { RangePicker } = DatePicker;

// 排产明细类型
interface PlanItem {
  key: string;
  version_id: string;
  material_id: string;
  machine_code: string;
  plan_date: string;
  seq_no: number;
  weight_t: number;
  steel_grade?: string;
  urgent_level?: string;
  source_type: string;
  locked_in_plan: boolean;
  force_release_in_plan: boolean;
  sched_state?: string;
  assign_reason?: string;
}

// 统计信息类型
interface Statistics {
  total_items: number;
  total_weight: number;
  by_machine: Record<string, number>;
  by_urgent_level: Record<string, number>;
  frozen_count: number;
}

// 可拖拽行组件
interface DraggableRowProps extends React.HTMLAttributes<HTMLTableRowElement> {
  'data-row-key': string;
}

const DraggableRow: React.FC<DraggableRowProps> = (props) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: props['data-row-key'],
  });

  const style: React.CSSProperties = {
    ...props.style,
    transform: CSS.Transform.toString(transform),
    transition,
    cursor: 'move',
    ...(isDragging ? { position: 'relative', zIndex: 9999 } : {}),
  };

  return <tr {...props} ref={setNodeRef} style={style} {...attributes} {...listeners} />;
};

interface PlanItemVisualizationProps {
  onNavigateToPlan?: () => void; // 导航到排产方案的回调
  machineCode?: string | null; // 外部控制机组筛选（'all' | 'H031'...）
  urgentLevel?: string | null; // 外部控制紧急度筛选（'all' | 'L0'...）
  refreshSignal?: number; // 变化时触发重新加载
  selectedMaterialIds?: string[]; // 外部控制选中项
  onSelectedMaterialIdsChange?: (ids: string[]) => void; // 选中项变化回调
}

const PlanItemVisualization: React.FC<PlanItemVisualizationProps> = ({
  onNavigateToPlan,
  machineCode,
  urgentLevel,
  refreshSignal,
  selectedMaterialIds: controlledSelectedMaterialIds,
  onSelectedMaterialIdsChange,
}) => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [planItems, setPlanItems] = useState<PlanItem[]>([]);
  const [filteredItems, setFilteredItems] = useState<PlanItem[]>([]);
  const [statistics, setStatistics] = useState<Statistics | null>(null);
  const [selectedMachine, setSelectedMachine] = useState<string>('all');
  const [selectedDate, setSelectedDate] = useState<Dayjs | null>(null);
  const [dateRange, setDateRange] = useState<[Dayjs, Dayjs] | null>(null);
  const [searchText, setSearchText] = useState('');
  const [selectedUrgentLevel, setSelectedUrgentLevel] = useState<string>('all');
  const [selectedItem, setSelectedItem] = useState<PlanItem | null>(null);
  const [showDetailModal, setShowDetailModal] = useState(false);
  const [internalSelectedMaterialIds, setInternalSelectedMaterialIds] = useState<string[]>([]);
  const selectedMaterialIds = controlledSelectedMaterialIds ?? internalSelectedMaterialIds;
  const setSelectedMaterialIds = (ids: string[]) => {
    if (onSelectedMaterialIdsChange) {
      onSelectedMaterialIdsChange(ids);
      return;
    }
    setInternalSelectedMaterialIds(ids);
  };
  const [forceReleaseModalVisible, setForceReleaseModalVisible] = useState(false);
  const [forceReleaseReason, setForceReleaseReason] = useState('');
  const [forceReleaseMode, setForceReleaseMode] = useState<'AutoFix' | 'Strict'>('AutoFix');
  const activeVersionId = useActiveVersionId();
  const currentUser = useCurrentUser();
  const navigateToPlan = onNavigateToPlan || (() => navigate('/comparison'));

  // 拖拽传感器
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 1,
      },
    })
  );

  // 订阅plan_updated事件,自动刷新
  useEvent('plan_updated', () => {
    if (activeVersionId) {
      loadPlanItems(activeVersionId);
    }
  });

  // 外部筛选控制
  useEffect(() => {
    setSelectedMachine(machineCode ?? 'all');
  }, [machineCode]);

  useEffect(() => {
    setSelectedUrgentLevel(urgentLevel ?? 'all');
  }, [urgentLevel]);

  // 紧急等级颜色映射
  const urgentLevelColors: Record<string, string> = {
    L3: 'red',
    L2: 'orange',
    L1: 'gold',
    L0: 'default',
  };

  // 来源类型标签
  const sourceTypeLabels: Record<string, { text: string; color: string }> = {
    CALC: { text: '计算', color: 'blue' },
    FROZEN: { text: '冻结', color: 'purple' },
    MANUAL: { text: '人工', color: 'green' },
  };

  // 加载排产明细
  const loadPlanItems = async (versionId?: string, date?: string) => {
    if (!versionId && !activeVersionId) {
      message.warning('请先激活一个版本');
      return;
    }

    const targetVersionId = versionId || activeVersionId;
    setLoading(true);
    try {
      let result;
      if (date) {
        result = await planApi.listItemsByDate(targetVersionId!, date);
      } else {
        result = await planApi.listPlanItems(targetVersionId!);
      }

      const items = (Array.isArray(result) ? result : []).map((item: any) => ({
        // PlanItem 在同一 version 内 material_id 唯一，可直接作为行 key / 选中 key / DnD key。
        key: String(item.material_id ?? ''),
        ...item,
      }));

      setPlanItems(items);
      setFilteredItems(items);
      calculateStatistics(items);
      message.success(`成功加载 ${items.length} 条排产明细`);
    } catch (error: any) {
      console.error('加载排产明细失败:', error);
      // 错误已由 IpcClient 统一处理
    } finally {
      setLoading(false);
    }
  };

  // 计算统计信息
  const calculateStatistics = (items: PlanItem[]) => {
    const stats: Statistics = {
      total_items: items.length,
      total_weight: items.reduce((sum, item) => sum + item.weight_t, 0),
      by_machine: {},
      by_urgent_level: {},
      frozen_count: items.filter((item) => item.locked_in_plan).length,
    };

    items.forEach((item) => {
      // 按机组统计
      stats.by_machine[item.machine_code] =
        (stats.by_machine[item.machine_code] || 0) + 1;

      // 按紧急等级统计
      if (item.urgent_level) {
        stats.by_urgent_level[item.urgent_level] =
          (stats.by_urgent_level[item.urgent_level] || 0) + 1;
      }
    });

    setStatistics(stats);
  };

  // 筛选数据
  const filterData = () => {
    let filtered = [...planItems];

    // 按机组筛选
    if (selectedMachine !== 'all') {
      filtered = filtered.filter((item) => item.machine_code === selectedMachine);
    }

    // 按日期筛选
    if (selectedDate) {
      const dateStr = formatDate(selectedDate);
      filtered = filtered.filter((item) => item.plan_date === dateStr);
    }

    // 按日期范围筛选
    if (dateRange) {
      const [start, end] = dateRange;
      filtered = filtered.filter((item) => {
        const itemDate = dayjs(item.plan_date);
        return itemDate.isAfter(start.subtract(1, 'day')) && itemDate.isBefore(end.add(1, 'day'));
      });
    }

    // 按搜索文本筛选
    if (searchText) {
      const searchLower = searchText.toLowerCase();
      filtered = filtered.filter(
        (item) =>
          item.material_id.toLowerCase().includes(searchLower) ||
          item.steel_grade?.toLowerCase().includes(searchLower)
      );
    }

    // 按紧急等级筛选
    if (selectedUrgentLevel !== 'all') {
      filtered = filtered.filter((item) => item.urgent_level === selectedUrgentLevel);
    }

    setFilteredItems(filtered);
    calculateStatistics(filtered);
  };

  // 查看详情
  const handleViewDetail = (item: PlanItem) => {
    setSelectedItem(item);
    setShowDetailModal(true);
  };

  // 批量强制放行
  const handleBatchForceRelease = async () => {
    if (!forceReleaseReason.trim()) {
      message.warning('请输入强制放行原因');
      return;
    }

    setLoading(true);
    try {
      const res: any = await materialApi.batchForceRelease(
        selectedMaterialIds,
        currentUser,
        forceReleaseReason,
        forceReleaseMode
      );
      message.success(String(res?.message || `成功强制放行 ${selectedMaterialIds.length} 个材料`));

      const violations = Array.isArray(res?.details?.violations) ? res.details.violations : [];
      if (violations.length > 0) {
        Modal.info({
          title: '强制放行警告（未适温材料）',
          width: 720,
          content: (
            <div style={{ maxHeight: 420, overflow: 'auto' }}>
              <pre style={{ fontSize: 12, whiteSpace: 'pre-wrap' }}>
                {JSON.stringify(violations, null, 2)}
              </pre>
            </div>
          ),
        });
      }
      setForceReleaseModalVisible(false);
      setForceReleaseReason('');
      setForceReleaseMode('AutoFix');
      setSelectedMaterialIds([]);
      // 重新加载数据
      if (activeVersionId) {
        await loadPlanItems(activeVersionId);
      }
    } catch (error: any) {
      console.error('强制放行失败:', error);
    } finally {
      setLoading(false);
    }
  };

  // 拖拽结束处理
  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event;

    if (!over || active.id === over.id) {
      return;
    }

    const oldIndex = filteredItems.findIndex((item) => item.key === active.id);
    const newIndex = filteredItems.findIndex((item) => item.key === over.id);

    if (oldIndex === -1 || newIndex === -1) {
      return;
    }

    const draggedItem = filteredItems[oldIndex];
    const targetItem = filteredItems[newIndex];

    // 检查是否冻结
    if (draggedItem.locked_in_plan) {
      message.warning('冻结材料不可调整顺序');
      return;
    }

    // 检查是否同一日期和机组
    if (
      draggedItem.plan_date !== targetItem.plan_date ||
      draggedItem.machine_code !== targetItem.machine_code
    ) {
      message.warning('只能在同一日期和机组内调整顺序');
      return;
    }

    // 乐观更新UI
    const newItems = arrayMove(filteredItems, oldIndex, newIndex);
    setFilteredItems(newItems);

    // 调用API
    try {
      await planApi.moveItems(
        activeVersionId!,
        [
          {
            material_id: draggedItem.material_id,
            to_date: targetItem.plan_date,
            to_seq: targetItem.seq_no,
            to_machine: targetItem.machine_code,
          },
        ],
        'AUTO_FIX',
        currentUser || 'admin',
        '拖拽调整顺序'
      );
      message.success('顺序调整成功');
      // 重新加载数据
      if (activeVersionId) {
        await loadPlanItems(activeVersionId);
      }
    } catch (error: any) {
      console.error('调整顺序失败:', error);
      // 恢复原顺序
      setFilteredItems(filteredItems);
    }
  };

  const machineOptions = useMemo(() => {
    const codes = new Set<string>();
    planItems.forEach((it) => {
      const code = String(it.machine_code ?? '').trim();
      if (code) codes.add(code);
    });
    return Array.from(codes).sort();
  }, [planItems]);

  // 表格列定义
  const columns: ColumnsType<PlanItem> = [
    {
      title: '',
      key: 'drag',
      width: 40,
      render: (_, record: PlanItem) =>
        !record.locked_in_plan ? (
          <HolderOutlined style={{ cursor: 'move', color: '#999' }} />
        ) : null,
    },
    {
      title: '序号',
      dataIndex: 'seq_no',
      key: 'seq_no',
      width: 70,
      sorter: (a, b) => a.seq_no - b.seq_no,
      render: (seq_no: number, record: PlanItem) => (
        <Space>
          {record.locked_in_plan && (
            <Tooltip title="冻结">
              <LockOutlined style={{ color: '#8c8c8c' }} />
            </Tooltip>
          )}
          <span>{seq_no}</span>
        </Space>
      ),
    },
    {
      title: '材料ID',
      dataIndex: 'material_id',
      key: 'material_id',
      width: 150,
      render: (text: string, record: PlanItem) => (
        <Button type="link" onClick={() => handleViewDetail(record)}>
          {text}
        </Button>
      ),
    },
    {
      title: '钢种',
      dataIndex: 'steel_grade',
      key: 'steel_grade',
      width: 100,
    },
    {
      title: '吨位',
      dataIndex: 'weight_t',
      key: 'weight_t',
      width: 100,
      sorter: (a, b) => a.weight_t - b.weight_t,
      render: (value: number) => formatWeight(value),
    },
    {
      title: '机组',
      dataIndex: 'machine_code',
      key: 'machine_code',
      width: 100,
      filters: machineOptions.map((code) => ({ text: code, value: code })),
      onFilter: (value, record) => record.machine_code === value,
    },
    {
      title: '排产日期',
      dataIndex: 'plan_date',
      key: 'plan_date',
      width: 120,
      sorter: (a, b) => a.plan_date.localeCompare(b.plan_date),
    },
    {
      title: '紧急等级',
      dataIndex: 'urgent_level',
      key: 'urgent_level',
      width: 100,
      render: (level: string) => (
        <Tag color={urgentLevelColors[level] || 'default'}>{level}</Tag>
      ),
      filters: [
        { text: 'L3-超紧急', value: 'L3' },
        { text: 'L2-紧急', value: 'L2' },
        { text: 'L1-较紧急', value: 'L1' },
        { text: 'L0-正常', value: 'L0' },
      ],
      onFilter: (value, record) => record.urgent_level === value,
    },
    {
      title: '来源',
      dataIndex: 'source_type',
      key: 'source_type',
      width: 100,
      render: (type: string) => {
        const label = sourceTypeLabels[type] || { text: type, color: 'default' };
        return <Tag color={label.color}>{label.text}</Tag>;
      },
    },
    {
      title: '状态',
      key: 'status',
      width: 120,
      render: (_, record: PlanItem) => (
        <Space size={4}>
          {record.locked_in_plan && <Tag color="purple">冻结</Tag>}
          {record.force_release_in_plan && <Tag color="orange">强制放行</Tag>}
          {!record.locked_in_plan && !record.force_release_in_plan && (
            <Tag color="green">正常</Tag>
          )}
        </Space>
      ),
    },
    {
      title: '操作',
      key: 'action',
      width: 150,
      fixed: 'right',
      render: (_, record: PlanItem) => (
        <Space size="small">
          <Button type="link" size="small" onClick={() => handleViewDetail(record)}>
            详情
          </Button>
          {!record.locked_in_plan && (
            <Tooltip title="拖拽调整顺序">
              <Button type="link" size="small" icon={<DragOutlined />} />
            </Tooltip>
          )}
        </Space>
      ),
    },
  ];

  // 初始加载
  useEffect(() => {
    if (activeVersionId) {
      loadPlanItems(activeVersionId);
    }
  }, [activeVersionId, refreshSignal]);

  // 筛选条件变化时重新筛选
  useEffect(() => {
    filterData();
  }, [selectedMachine, selectedDate, dateRange, searchText, selectedUrgentLevel]);

  // 没有激活版本时显示引导
  if (!activeVersionId) {
    return (
      <NoActiveVersionGuide
        title="尚无激活的排产版本"
        description="排产明细可视化需要一个激活的排产版本作为基础"
        onNavigateToPlan={navigateToPlan}
      />
    );
  }

  return (
    <div style={{ padding: '24px' }}>
      {/* 标题和操作栏 */}
      <Row justify="space-between" align="middle" style={{ marginBottom: 16 }}>
        <Col>
          <h2 style={{ margin: 0 }}>排产明细可视化</h2>
        </Col>
        <Col>
          <Space>
            <Button icon={<ReloadOutlined />} onClick={() => loadPlanItems()}>
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
                        const data = filteredItems.map((item) => ({
                          材料号: item.material_id,
                          机组: item.machine_code,
                          计划日期: item.plan_date,
                          序号: item.seq_no,
                          重量: formatWeight(item.weight_t),
                          钢种: item.steel_grade || '-',
                          紧急等级: item.urgent_level || '-',
                          来源: item.source_type,
                          锁定: item.locked_in_plan ? '是' : '否',
                          强制放行: item.force_release_in_plan ? '是' : '否',
                          排产状态: item.sched_state || '-',
                          原因: item.assign_reason || '-',
                        }));
                        exportCSV(data, '排产明细');
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
                        exportJSON(filteredItems, '排产明细');
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

      {/* 统计卡片 */}
      {statistics && (
        <Row gutter={16} style={{ marginBottom: 16 }}>
          <Col span={6}>
            <Card>
              <Statistic
                title="总排产数"
                value={statistics.total_items}
                suffix="个"
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card>
              <Statistic
                title="总吨位"
                value={statistics.total_weight.toFixed(1)}
                suffix="吨"
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card>
              <Statistic
                title="冻结材料"
                value={statistics.frozen_count}
                suffix="个"
                valueStyle={{ color: '#8c8c8c' }}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card>
              <Statistic
                title="紧急材料(L2+)"
                value={
                  (statistics.by_urgent_level['L2'] || 0) +
                  (statistics.by_urgent_level['L3'] || 0)
                }
                suffix="个"
                valueStyle={{ color: '#cf1322' }}
              />
            </Card>
          </Col>
        </Row>
      )}

      {/* 筛选栏 */}
      <Card style={{ marginBottom: 16 }}>
        <Space wrap>
          <Select
            style={{ width: 150 }}
            placeholder="选择机组"
            value={selectedMachine}
            onChange={setSelectedMachine}
          >
            <Option value="all">全部机组</Option>
            {machineOptions.map((code) => (
              <Option key={code} value={code}>
                {code}
              </Option>
            ))}
          </Select>

          <Select
            style={{ width: 150 }}
            placeholder="紧急等级"
            value={selectedUrgentLevel}
            onChange={setSelectedUrgentLevel}
          >
            <Option value="all">全部等级</Option>
            <Option value="L3">L3-超紧急</Option>
            <Option value="L2">L2-紧急</Option>
            <Option value="L1">L1-较紧急</Option>
            <Option value="L0">L0-正常</Option>
          </Select>

          <DatePicker
            placeholder="选择日期"
            value={selectedDate}
            onChange={setSelectedDate}
            format="YYYY-MM-DD"
          />

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

          <Search
            placeholder="搜索材料ID或钢种"
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 250 }}
            allowClear
          />

          <Button
            icon={<FilterOutlined />}
            onClick={() => {
              setSelectedMachine('all');
              setSelectedUrgentLevel('all');
              setSelectedDate(null);
              setDateRange(null);
              setSearchText('');
            }}
          >
            清除筛选
          </Button>
        </Space>
      </Card>

      {/* 批量操作栏 */}
      {selectedMaterialIds.length > 0 && (
        <Card style={{ marginBottom: 16, backgroundColor: '#e6f7ff' }}>
          <Space>
            <span>已选择 {selectedMaterialIds.length} 个材料</span>
            <Button
              type="primary"
              onClick={() => setForceReleaseModalVisible(true)}
            >
              批量强制放行
            </Button>
            <Button onClick={() => setSelectedMaterialIds([])}>取消选择</Button>
          </Space>
        </Card>
      )}

      {/* 排产明细表格 */}
      <Card>
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={handleDragEnd}
        >
          <SortableContext
            items={filteredItems.map((item) => item.key)}
            strategy={verticalListSortingStrategy}
          >
            <Table
              columns={columns}
              dataSource={filteredItems}
              loading={loading}
              locale={tableFilterEmptyConfig}
              rowKey="material_id"
              pagination={{
                pageSize: 20,
                showSizeChanger: true,
                showTotal: (total) => `共 ${total} 条记录`,
              }}
              scroll={{ x: 1400 }}
              size="small"
              rowSelection={{
                type: 'checkbox',
                selectedRowKeys: selectedMaterialIds,
                onChange: (selectedKeys) => {
                  setSelectedMaterialIds(selectedKeys as string[]);
                },
                getCheckboxProps: (record) => ({
                  disabled: record.locked_in_plan || record.force_release_in_plan,
                }),
              }}
              components={{
                body: {
                  row: DraggableRow,
                },
              }}
            />
          </SortableContext>
        </DndContext>
      </Card>

      {/* 详情模态框 */}
      <Modal
        title="排产明细详情"
        open={showDetailModal}
        onCancel={() => setShowDetailModal(false)}
        footer={[
          <Button key="close" onClick={() => setShowDetailModal(false)}>
            关闭
          </Button>,
        ]}
        width={700}
      >
        {selectedItem && (
          <Descriptions bordered column={2}>
            <Descriptions.Item label="材料ID" span={2}>
              {selectedItem.material_id}
            </Descriptions.Item>
            <Descriptions.Item label="钢种">
              {selectedItem.steel_grade}
            </Descriptions.Item>
            <Descriptions.Item label="吨位">
              {formatWeight(selectedItem.weight_t)}
            </Descriptions.Item>
            <Descriptions.Item label="机组">
              {selectedItem.machine_code}
            </Descriptions.Item>
            <Descriptions.Item label="排产日期">
              {selectedItem.plan_date}
            </Descriptions.Item>
            <Descriptions.Item label="序号">
              {selectedItem.seq_no}
            </Descriptions.Item>
            <Descriptions.Item label="紧急等级">
              <Tag color={urgentLevelColors[selectedItem.urgent_level || 'L0']}>
                {selectedItem.urgent_level}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="来源类型">
              <Tag
                color={
                  sourceTypeLabels[selectedItem.source_type]?.color || 'default'
                }
              >
                {sourceTypeLabels[selectedItem.source_type]?.text ||
                  selectedItem.source_type}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="状态" span={2}>
              <Space>
                {selectedItem.locked_in_plan && <Tag color="purple">冻结</Tag>}
                {selectedItem.force_release_in_plan && (
                  <Tag color="orange">强制放行</Tag>
                )}
                {!selectedItem.locked_in_plan &&
                  !selectedItem.force_release_in_plan && (
                    <Tag color="green">正常</Tag>
                  )}
              </Space>
            </Descriptions.Item>
            <Descriptions.Item label="排产状态">
              {selectedItem.sched_state}
            </Descriptions.Item>
            <Descriptions.Item label="落位原因" span={2}>
              {selectedItem.assign_reason}
            </Descriptions.Item>
          </Descriptions>
        )}
      </Modal>

      {/* 强制放行模态框 */}
      <Modal
        title="批量强制放行"
        open={forceReleaseModalVisible}
        onOk={handleBatchForceRelease}
        onCancel={() => {
          setForceReleaseModalVisible(false);
          setForceReleaseReason('');
          setForceReleaseMode('AutoFix');
        }}
        confirmLoading={loading}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <p>即将强制放行 {selectedMaterialIds.length} 个材料</p>
          <Space wrap>
            <span>校验模式</span>
            <Select
              value={forceReleaseMode}
              onChange={(v) => setForceReleaseMode(v as 'AutoFix' | 'Strict')}
              style={{ width: 220 }}
            >
              <Option value="AutoFix">AUTO_FIX（允许未适温）</Option>
              <Option value="Strict">STRICT（未适温则失败）</Option>
            </Select>
          </Space>
          <p style={{ margin: 0, fontSize: 12, color: '#8c8c8c' }}>
            提示：STRICT 遇到未适温材料会失败；AUTO_FIX 允许放行并记录警告（可审计）。
          </p>
          <Input.TextArea
            placeholder="请输入强制放行原因(必填)"
            value={forceReleaseReason}
            onChange={(e) => setForceReleaseReason(e.target.value)}
            rows={4}
          />
        </Space>
      </Modal>
    </div>
  );
};

export default PlanItemVisualization;
