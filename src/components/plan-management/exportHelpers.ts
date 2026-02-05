/**
 * PlanManagement 导出相关函数
 */

import dayjs from 'dayjs';
import { message } from 'antd';
import { exportCSV, exportJSON, exportHTML, exportMarkdown } from '../../utils/exportUtils';
import type { ExportContext, ConfigChange } from './types';

export type { ExportContext } from './types';

export const exportCapacityDelta = async (
  format: 'csv' | 'json',
  context: ExportContext
): Promise<void> => {
  const { compareResult, localCapacityRows } = context;
  if (!compareResult || !localCapacityRows) return;

  const rows = localCapacityRows.rows.map((r) => ({
    date: r.date,
    machine_code: r.machine_code,
    used_a: r.used_a,
    used_b: r.used_b,
    delta: r.delta,
    target_a: r.target_a,
    limit_a: r.limit_a,
    target_b: r.target_b,
    limit_b: r.limit_b,
  }));

  const filename = `产能差异_${compareResult.version_id_a}_vs_${compareResult.version_id_b}`;
  if (format === 'csv') exportCSV(rows, filename);
  else exportJSON(rows, filename);
};

export const exportDiffs = async (
  format: 'csv' | 'json',
  context: ExportContext
): Promise<void> => {
  const { compareResult, localDiffResult } = context;
  if (!compareResult || !localDiffResult) return;

  const rows = localDiffResult.diffs.map((d) => ({
    change_type: d.changeType,
    material_id: d.materialId,
    from_machine: d.previousState?.machine_code ?? null,
    from_date: d.previousState?.plan_date ?? null,
    from_seq: d.previousState?.seq_no ?? null,
    to_machine: d.currentState?.machine_code ?? null,
    to_date: d.currentState?.plan_date ?? null,
    to_seq: d.currentState?.seq_no ?? null,
    weight_t: d.currentState?.weight_t ?? d.previousState?.weight_t ?? null,
    urgent_level: d.currentState?.urgent_level ?? d.previousState?.urgent_level ?? null,
    locked_in_plan: d.currentState?.locked_in_plan ?? d.previousState?.locked_in_plan ?? null,
    force_release_in_plan: d.currentState?.force_release_in_plan ?? d.previousState?.force_release_in_plan ?? null,
  }));

  const filename = `版本差异_${compareResult.version_id_a}_vs_${compareResult.version_id_b}`;
  if (format === 'csv') exportCSV(rows, filename);
  else exportJSON(rows, filename);
};

export const exportRetrospectiveReport = async (context: ExportContext): Promise<void> => {
  const { compareResult, currentUser, retrospectiveNote } = context;
  if (!compareResult) return;

  try {
    const json = JSON.stringify(
      {
        timestamp: dayjs().format('YYYY-MM-DD HH:mm:ss'),
        operator: currentUser,
        version_id_a: compareResult.version_id_a,
        version_id_b: compareResult.version_id_b,
        retrospective_note: retrospectiveNote,
      },
      null,
      2
    );
    exportJSON(JSON.parse(json), '复盘总结');
    message.success('已导出复盘总结（JSON）');
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : '导出失败';
    message.error(errorMessage);
  }
};

export const exportReportMarkdown = async (context: ExportContext): Promise<void> => {
  const { compareResult, currentUser, localDiffResult, localCapacityRows, retrospectiveNote } = context;
  if (!compareResult) return;

  const header = `# 版本对比报告\n\n- 导出时间：${dayjs().format('YYYY-MM-DD HH:mm:ss')}\n- 操作人：${currentUser}\n- 版本A：${compareResult.version_id_a}\n- 版本B：${compareResult.version_id_b}\n\n`;
  const backendSummary = `## 后端摘要\n\n- moved_count: ${compareResult.moved_count}\n- added_count: ${compareResult.added_count}\n- removed_count: ${compareResult.removed_count}\n- squeezed_out_count: ${compareResult.squeezed_out_count}\n\n`;

  const localSummary = localDiffResult
    ? `## 本地差异摘要（由排产明细计算）\n\n- totalChanges: ${localDiffResult.summary.totalChanges}\n- movedCount: ${localDiffResult.summary.movedCount}\n- modifiedCount: ${localDiffResult.summary.modifiedCount}\n- addedCount: ${localDiffResult.summary.addedCount}\n- removedCount: ${localDiffResult.summary.removedCount}\n\n`
    : `## 本地差异摘要（由排产明细计算）\n\n> 本地差异明细未加载完成或加载失败。\n\n`;

  const configChanges = (Array.isArray(compareResult.config_changes) ? compareResult.config_changes : []) as ConfigChange[];
  const configSection =
    configChanges.length > 0
      ? `## 配置变化\n\n| Key | 版本A | 版本B |\n| --- | --- | --- |\n${configChanges
          .map((c) => `| ${String(c.key)} | ${c.value_a == null ? '-' : String(c.value_a)} | ${c.value_b == null ? '-' : String(c.value_b)} |`)
          .join('\n')}\n\n`
      : `## 配置变化\n\n- 无配置变化\n\n`;

  const diffSample = localDiffResult ? localDiffResult.diffs.slice(0, 200) : [];
  const diffsSection =
    diffSample.length > 0
      ? `## 物料变更明细（示例前200条）\n\n| 类型 | 物料 | From | To |\n| --- | --- | --- | --- |\n${diffSample
          .map((d) => {
            const from = d.previousState ? `${d.previousState.machine_code}/${d.previousState.plan_date}/序${d.previousState.seq_no}` : '-';
            const to = d.currentState ? `${d.currentState.machine_code}/${d.currentState.plan_date}/序${d.currentState.seq_no}` : '-';
            return `| ${d.changeType} | ${d.materialId} | ${from} | ${to} |`;
          })
          .join('\n')}\n\n`
      : `## 物料变更明细\n\n- 无变更或未加载。\n\n`;

  const capacitySection = localCapacityRows
    ? `## 产能变化（本地计算）\n\n- 总量A: ${localCapacityRows.totalA.toFixed(2)}t\n- 总量B: ${localCapacityRows.totalB.toFixed(2)}t\n- Δ: ${(localCapacityRows.totalB - localCapacityRows.totalA).toFixed(2)}t\n- 预计超上限行数（按版本B产能池）：${localCapacityRows.overflowRows.length}\n\n`
    : `## 产能变化（本地计算）\n\n- 未加载。\n\n`;

  const retrospectiveSection = `## 复盘总结（本地）\n\n${retrospectiveNote.trim() || '（空）'}\n\n`;

  try {
    exportMarkdown(header + backendSummary + localSummary + configSection + diffsSection + capacitySection + retrospectiveSection, '版本对比报告');
    message.success('已导出（Markdown）');
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : '导出失败';
    message.error(errorMessage);
  }
};

export const exportReportHTML = async (context: ExportContext): Promise<void> => {
  const { compareResult, currentUser, localDiffResult, localCapacityRows, retrospectiveNote } = context;
  if (!compareResult) return;

  const configChanges = (Array.isArray(compareResult.config_changes) ? compareResult.config_changes : []) as ConfigChange[];
  const diffSample = localDiffResult ? localDiffResult.diffs.slice(0, 200) : [];

  const escape = (v: unknown) =>
    String(v ?? '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\"/g, '&quot;');

  const html = `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>版本对比报告</title>
    <style>
      body { font-family: -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica,Arial,"PingFang SC","Hiragino Sans GB","Microsoft YaHei",sans-serif; padding: 24px; }
      h1,h2 { margin: 16px 0 8px; }
      .meta { color: #666; font-size: 13px; margin-bottom: 16px; }
      table { border-collapse: collapse; width: 100%; margin: 8px 0 16px; }
      th, td { border: 1px solid #eee; padding: 8px; font-size: 13px; text-align: left; }
      th { background: #fafafa; }
      code { font-family: ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace; }
      .badge { display: inline-block; padding: 2px 8px; border-radius: 999px; font-size: 12px; }
      .badge.red { background: rgba(255,77,79,0.15); color: #cf1322; }
      .badge.blue { background: rgba(22,119,255,0.12); color: #1677ff; }
    </style>
  </head>
  <body>
    <h1>版本对比报告</h1>
    <div class="meta">
      导出时间：${escape(dayjs().format('YYYY-MM-DD HH:mm:ss'))} · 操作人：${escape(currentUser)}<br/>
      版本A：<code>${escape(compareResult.version_id_a)}</code> · 版本B：<code>${escape(compareResult.version_id_b)}</code>
    </div>

    <h2>后端摘要</h2>
    <table>
      <tr><th>moved_count</th><td>${escape(compareResult.moved_count)}</td><th>added_count</th><td>${escape(compareResult.added_count)}</td></tr>
      <tr><th>removed_count</th><td>${escape(compareResult.removed_count)}</td><th>squeezed_out_count</th><td>${escape(compareResult.squeezed_out_count)}</td></tr>
    </table>

    <h2>本地差异摘要（由排产明细计算）</h2>
    ${
      localDiffResult
        ? `<table>
      <tr><th>totalChanges</th><td>${escape(localDiffResult.summary.totalChanges)}</td><th>movedCount</th><td>${escape(localDiffResult.summary.movedCount)}</td></tr>
      <tr><th>modifiedCount</th><td>${escape(localDiffResult.summary.modifiedCount)}</td><th>addedCount</th><td>${escape(localDiffResult.summary.addedCount)}</td></tr>
      <tr><th>removedCount</th><td>${escape(localDiffResult.summary.removedCount)}</td><th></th><td></td></tr>
    </table>`
        : `<div class="meta">本地差异明细未加载完成或加载失败。</div>`
    }

    <h2>配置变化</h2>
    ${
      configChanges.length > 0
        ? `<table>
      <thead><tr><th>Key</th><th>版本A</th><th>版本B</th></tr></thead>
      <tbody>
        ${configChanges
          .map((c) => `<tr><td>${escape(c.key)}</td><td>${escape(c.value_a ?? '-')}</td><td>${escape(c.value_b ?? '-')}</td></tr>`)
          .join('')}
      </tbody>
    </table>`
        : `<div class="meta">无配置变化</div>`
    }

    <h2>物料变更明细（示例前200条）</h2>
    ${
      diffSample.length > 0
        ? `<table>
      <thead><tr><th>类型</th><th>物料</th><th>From</th><th>To</th></tr></thead>
      <tbody>
        ${diffSample
          .map((d) => {
            const from = d.previousState ? `${d.previousState.machine_code}/${d.previousState.plan_date}/序${d.previousState.seq_no}` : '-';
            const to = d.currentState ? `${d.currentState.machine_code}/${d.currentState.plan_date}/序${d.currentState.seq_no}` : '-';
            return `<tr>
              <td><span class="badge ${d.changeType === 'REMOVED' ? 'red' : 'blue'}">${escape(d.changeType)}</span></td>
              <td><code>${escape(d.materialId)}</code></td>
              <td>${escape(from)}</td>
              <td>${escape(to)}</td>
            </tr>`;
          })
          .join('')}
      </tbody>
    </table>`
        : `<div class="meta">无变更或未加载。</div>`
    }

    <h2>产能变化（本地计算）</h2>
    ${
      localCapacityRows
        ? `<table>
      <tr><th>总量A</th><td>${escape(localCapacityRows.totalA.toFixed(2))}t</td><th>总量B</th><td>${escape(localCapacityRows.totalB.toFixed(2))}t</td></tr>
      <tr><th>Δ</th><td>${escape((localCapacityRows.totalB - localCapacityRows.totalA).toFixed(2))}t</td><th>预计超上限行数</th><td>${escape(localCapacityRows.overflowRows.length)}</td></tr>
    </table>`
        : `<div class="meta">未加载。</div>`
    }

    <h2>复盘总结（本地）</h2>
    <pre style="white-space: pre-wrap; border: 1px solid #eee; background: #fafafa; padding: 12px; border-radius: 6px;">${escape(
      retrospectiveNote.trim() || '（空）'
    )}</pre>
  </body>
</html>`;

  try {
    exportHTML(html, '版本对比报告');
    message.success('已导出（HTML）');
  } catch (e) {
    const errorMessage = e instanceof Error ? e.message : '导出失败';
    message.error(errorMessage);
  }
};
