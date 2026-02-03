/**
 * Workbench 消息通知统一接口
 *
 * 目标：统一 message + Modal.info/confirm 的反馈链路
 *
 * 原来的实现分散在各处：
 * - message.warning('请先选择物料');
 * - message.success('推荐位置：...');
 * - message.error(`推荐位置失败: ${error}`);
 * - Modal.info({ title, content: <...> });
 *
 * 重构后：
 * - const notify = useWorkbenchNotification();
 * - notify.operationSuccess('锁定', ids.length);
 * - notify.operationError('锁定', error);
 * - notify.asyncResultDetail('移动结果', <...>);
 */

import { message, Modal } from 'antd';
import React from 'react';

/**
 * 从错误对象中提取消息
 */
function getErrorMessage(error: unknown): string {
  if (!error) return '未知错误';
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message;
  if (typeof error === 'object' && error !== null) {
    const obj = error as Record<string, unknown>;
    if (typeof obj.message === 'string') return obj.message;
    if (typeof obj.msg === 'string') return obj.msg;
    if (typeof obj.error === 'string') return obj.error;
  }
  return String(error);
}

export function useWorkbenchNotification() {
  /**
   * 操作成功反馈
   *
   * @param operation 操作名称（例如："锁定"、"移动"、"设置紧急标志"）
   * @param count 可选的操作数量
   *
   * @example
   * notify.operationSuccess('锁定', ids.length);
   * // → 显示："锁定成功（3个）"
   */
  const operationSuccess = (operation: string, count?: number) => {
    const countText = typeof count === 'number' && count > 0 ? `（${count}个）` : '';
    message.success(`${operation}成功${countText}`);
  };

  /**
   * 操作失败反馈
   *
   * @param operation 操作名称
   * @param error 错误对象
   *
   * @example
   * notify.operationError('锁定', error);
   * // → 显示："锁定失败：{errorMessage}"
   */
  const operationError = (operation: string, error?: unknown) => {
    const errorMessage = getErrorMessage(error);
    message.error(`${operation}失败：${errorMessage}`);
  };

  /**
   * 前置校验失败反馈（警告）
   *
   * @param reason 失败原因
   *
   * @example
   * notify.validationFail('请先选择物料');
   * // → 显示警告消息
   */
  const validationFail = (reason: string) => {
    message.warning(reason);
  };

  /**
   * 异步结果详情展示（Modal.info）
   *
   * @param title 标题
   * @param content 内容（React 节点）
   *
   * @example
   * notify.asyncResultDetail('移动结果', <Table dataSource={...} />);
   * // → 弹出信息弹窗
   */
  const asyncResultDetail = (title: string, content: React.ReactNode) => {
    Modal.info({
      title,
      width: 920,
      content,
    });
  };

  /**
   * 通用信息提示
   *
   * @param text 消息文本
   *
   * @example
   * notify.info('有 3 个物料不在排程中，将跳过移动');
   */
  const info = (text: string) => {
    message.info(text);
  };

  /**
   * 通用成功提示
   *
   * @param text 消息文本
   */
  const success = (text: string) => {
    message.success(text);
  };

  /**
   * 通用警告提示
   *
   * @param text 消息文本
   */
  const warning = (text: string) => {
    message.warning(text);
  };

  /**
   * 通用错误提示
   *
   * @param text 消息文本
   */
  const error = (text: string) => {
    message.error(text);
  };

  return {
    // 专用方法（推荐）
    operationSuccess,
    operationError,
    validationFail,
    asyncResultDetail,

    // 通用方法（向后兼容）
    info,
    success,
    warning,
    error,
  };
}
