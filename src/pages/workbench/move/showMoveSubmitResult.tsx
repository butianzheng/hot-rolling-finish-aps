import { Alert, Modal, Space, Table, message } from 'antd';

import type { MoveItemResultRow } from '../types';

type MoveItemsResponseLike = {
  failed_count?: unknown;
  message?: unknown;
  results?: unknown;
};

export function showMoveSubmitResult(res: MoveItemsResponseLike, missing: string[]) {
  const failedCount = Number(res?.failed_count ?? 0);
  const successCount = Number((res as { success_count?: unknown })?.success_count ?? 0);
  const movedHint = successCount > 0 ? '（已更新当前工作版本，决策同步中）' : '';

  if (failedCount > 0) {
    const rawResults = Array.isArray(res?.results) ? res.results : [];
    const results: MoveItemResultRow[] = rawResults.map((r: any) => ({
      material_id: String(r?.material_id ?? ''),
      success: Boolean(r?.success),
      from_machine: r?.from_machine == null ? null : String(r.from_machine),
      from_date: r?.from_date == null ? null : String(r.from_date),
      to_machine: String(r?.to_machine ?? ''),
      to_date: String(r?.to_date ?? ''),
      error: r?.error == null ? null : String(r.error),
      violation_type: r?.violation_type == null ? null : String(r.violation_type),
    }));

    Modal.info({
      title: '移动完成（部分失败）',
      width: 920,
      content: (
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          <Alert type="warning" showIcon message={`${String(res?.message || '移动完成')}${movedHint}`} />
          {missing.length > 0 && <Alert type="info" showIcon message={`有 ${missing.length} 个物料不在当前版本排程中，已跳过`} />}
          <Table<MoveItemResultRow>
            size="small"
            rowKey={(r) => r.material_id}
            pagination={false}
            dataSource={results}
            columns={[
              { title: '物料', dataIndex: 'material_id', width: 160 },
              {
                title: '结果',
                dataIndex: 'success',
                width: 80,
                render: (v) => (v ? '成功' : '失败'),
              },
              {
                title: '原位置',
                key: 'from',
                width: 220,
                render: (_, r) => `${r.from_machine || '-'}/${r.from_date || '-'}`,
              },
              {
                title: '目标位置',
                key: 'to',
                width: 220,
                render: (_, r) => `${r.to_machine || '-'}/${r.to_date || '-'}`,
              },
              { title: '原因', dataIndex: 'error' },
            ]}
            scroll={{ y: 320 }}
          />
        </Space>
      ),
    });
    return;
  }

  message.success(`${String(res?.message || '移动完成')}${movedHint}`);
  if (missing.length > 0) {
    message.info(`有 ${missing.length} 个物料不在当前版本排程中，已跳过`);
  }
}
