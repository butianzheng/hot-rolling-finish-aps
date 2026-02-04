/**
 * 风险快照筛选栏
 */

import React from 'react';
import { Button, Col, DatePicker, Dropdown, message, Row, Select, Space } from 'antd';
import { DownloadOutlined, ReloadOutlined } from '@ant-design/icons';
import type { Dayjs } from 'dayjs';
import { formatNumber } from '../../utils/formatters';
import { exportCSV, exportJSON } from '../../utils/exportUtils';
import type { DaySummary } from '../../types/decision';
import type { VersionOption } from './types';

const { Option } = Select;
const { RangePicker } = DatePicker;

export interface FilterBarProps {
  versionOptions: VersionOption[];
  selectedVersion: string;
  onVersionChange: (version: string) => void;
  dateRange: [Dayjs, Dayjs] | null;
  onDateRangeChange: (range: [Dayjs, Dayjs] | null) => void;
  onRefresh: () => void;
  riskSnapshots: DaySummary[];
}

export const FilterBar: React.FC<FilterBarProps> = ({
  versionOptions,
  selectedVersion,
  onVersionChange,
  dateRange,
  onDateRangeChange,
  onRefresh,
  riskSnapshots,
}) => {
  const handleExportCSV = () => {
    try {
      const data = riskSnapshots.map((snapshot) => ({
        日期: snapshot.planDate,
        风险分数: formatNumber(snapshot.riskScore, 1),
        风险等级: snapshot.riskLevel,
        产能利用率: formatNumber(snapshot.capacityUtilPct, 1),
        超载吨数: formatNumber(snapshot.overloadWeightT, 1),
        紧急失败数: snapshot.urgentFailureCount,
        涉及机组: (snapshot.involvedMachines || []).join(','),
      }));
      exportCSV(data, '风险摘要(D1)');
      message.success('导出成功');
    } catch (error: any) {
      message.error(`导出失败: ${error.message}`);
    }
  };

  const handleExportJSON = () => {
    try {
      exportJSON(riskSnapshots as unknown as Record<string, unknown>[], '风险快照');
      message.success('导出成功');
    } catch (error: any) {
      message.error(`导出失败: ${error.message}`);
    }
  };

  return (
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
            onChange={onVersionChange}
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
                onDateRangeChange([dates[0], dates[1]]);
              } else {
                onDateRangeChange(null);
              }
            }}
            format="YYYY-MM-DD"
          />
          <Button icon={<ReloadOutlined />} onClick={onRefresh}>
            刷新
          </Button>
          <Dropdown
            menu={{
              items: [
                {
                  label: '导出为 CSV',
                  key: 'csv',
                  onClick: handleExportCSV,
                },
                {
                  label: '导出为 JSON',
                  key: 'json',
                  onClick: handleExportJSON,
                },
              ],
            }}
          >
            <Button icon={<DownloadOutlined />}>导出</Button>
          </Dropdown>
        </Space>
      </Col>
    </Row>
  );
};

export default FilterBar;
