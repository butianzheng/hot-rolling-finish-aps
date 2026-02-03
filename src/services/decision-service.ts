// ==========================================
// 决策服务层 - D1-D6 API封装
// ==========================================
// 职责: 封装决策API调用，集成Zod运行时验证
// 红线: 确保所有API响应通过Zod验证，保证类型安全
// 注意: 此服务负责 snake_case ↔ camelCase 转换
// ==========================================

import { IpcClient } from '../api/ipcClient';
import {
  DecisionDaySummaryResponseSchema,
  MachineBottleneckProfileResponseSchema,
  OrderFailureSetResponseSchema,
  ColdStockProfileResponseSchema,
  RollCampaignAlertResponseSchema,
  CapacityOpportunityResponseSchema,
  ErrorResponseSchema,
} from '../types/schemas';
import type {
  GetDecisionDaySummaryRequest,
  DecisionDaySummaryResponse,
  GetMachineBottleneckProfileRequest,
  MachineBottleneckProfileResponse,
  ListOrderFailureSetRequest,
  OrderFailureSetResponse,
  GetColdStockProfileRequest,
  ColdStockProfileResponse,
  GetRollCampaignAlertRequest,
  RollCampaignAlertResponse,
  GetCapacityOpportunityRequest,
  CapacityOpportunityResponse,
} from '../types/decision';

// ==========================================
// 转换工具函数
// ==========================================

/**
 * camelCase 转 snake_case
 */
function toSnakeCase(str: string): string {
  return str.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);
}

/**
 * snake_case 转 camelCase
 */
function toCamelCase(str: string): string {
  return str.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

/**
 * 递归将对象键从 camelCase 转换为 snake_case
 */
function objectToSnakeCase(obj: any): any {
  if (obj === null || obj === undefined) return obj;
  if (Array.isArray(obj)) return obj.map(objectToSnakeCase);
  if (typeof obj !== 'object') return obj;

  const result: Record<string, any> = {};
  for (const [key, value] of Object.entries(obj)) {
    result[toSnakeCase(key)] = objectToSnakeCase(value);
  }
  return result;
}

/**
 * 递归将对象键从 snake_case 转换为 camelCase
 */
function objectToCamelCase(obj: any): any {
  if (obj === null || obj === undefined) return obj;
  if (Array.isArray(obj)) return obj.map(objectToCamelCase);
  if (typeof obj !== 'object') return obj;

  const result: Record<string, any> = {};
  for (const [key, value] of Object.entries(obj)) {
    result[toCamelCase(key)] = objectToCamelCase(value);
  }
  return result;
}

/**
 * Tauri 命令层为了兼容 Rust 端的参数解析，部分数组参数使用 JSON 字符串传递：
 * - machine_codes
 * - *_filter
 *
 * 例如 Rust 侧签名为 `Option<String>`，再通过 `serde_json::from_str` 解析为 Vec<String>。
 */
function normalizeTauriParams(params: Record<string, any>): Record<string, any> {
  for (const [key, value] of Object.entries(params)) {
    if (
      Array.isArray(value) &&
      (key === 'machine_codes' || key.endsWith('_filter'))
    ) {
      params[key] = JSON.stringify(value);
    }
  }
  return params;
}

// ==========================================
// 错误处理
// ==========================================

/**
 * 决策API错误
 */
export class DecisionApiError extends Error {
  constructor(
    public code: string,
    message: string,
    public details?: any
  ) {
    super(message);
    this.name = 'DecisionApiError';
  }
}

/**
 * 验证错误（Zod验证失败）
 */
export class ValidationError extends Error {
  constructor(
    message: string,
    public zodError: any
  ) {
    super(message);
    this.name = 'ValidationError';
  }
}

/**
 * 通用API调用封装（带Zod验证和类型转换）
 * @param commandName Tauri命令名称
 * @param params 请求参数（camelCase）
 * @param schema Zod验证模式（snake_case）
 */
async function callWithValidation<T>(
  commandName: string,
  params: any,
  schema: any
): Promise<T> {
  try {
    // 将 camelCase 参数转换为 snake_case
    const snakeCaseParams = normalizeTauriParams(objectToSnakeCase(params));

    // 调用Tauri命令
    const rawResponse = await IpcClient.call(commandName, snakeCaseParams);

    // Zod运行时验证
    const parseResult = schema.safeParse(rawResponse);

    if (!parseResult.success) {
      console.error(`[DecisionService] Validation failed for ${commandName}:`, {
        rawResponse,
        zodError: parseResult.error,
      });

      throw new ValidationError(
        `响应数据验证失败: ${commandName}`,
        parseResult.error
      );
    }

    // 将 snake_case 响应转换为 camelCase
    return objectToCamelCase(parseResult.data) as T;
  } catch (error: any) {
    // 如果是已知错误，直接抛出
    if (error instanceof ValidationError || error instanceof DecisionApiError) {
      throw error;
    }

    // 尝试解析为错误响应
    const errorParseResult = ErrorResponseSchema.safeParse(error);
    if (errorParseResult.success) {
      const errorData = errorParseResult.data;
      throw new DecisionApiError(errorData.code, errorData.message, errorData.details);
    }

    // 未知错误
    throw new DecisionApiError(
      'UNKNOWN_ERROR',
      error.message || '未知错误',
      error
    );
  }
}

// ==========================================
// D1: 日期风险摘要 API
// ==========================================

/**
 * 获取日期风险摘要（D1）
 */
export async function getDecisionDaySummary(
  request: GetDecisionDaySummaryRequest
): Promise<DecisionDaySummaryResponse> {
  return callWithValidation<DecisionDaySummaryResponse>(
    'get_decision_day_summary',
    request,
    DecisionDaySummaryResponseSchema
  );
}

// ==========================================
// D4: 机组堵塞概况 API
// ==========================================

/**
 * 获取机组堵塞概况（D4）
 */
export async function getMachineBottleneckProfile(
  request: GetMachineBottleneckProfileRequest
): Promise<MachineBottleneckProfileResponse> {
  return callWithValidation<MachineBottleneckProfileResponse>(
    'get_machine_bottleneck_profile',
    request,
    MachineBottleneckProfileResponseSchema
  );
}

// ==========================================
// D2: 订单失败集合 API
// ==========================================

/**
 * 获取订单失败集合（D2）
 */
export async function listOrderFailureSet(
  request: ListOrderFailureSetRequest
): Promise<OrderFailureSetResponse> {
  return callWithValidation<OrderFailureSetResponse>(
    'list_order_failure_set',
    request,
    OrderFailureSetResponseSchema
  );
}

// ==========================================
// D3: 冷料压库概况 API
// ==========================================

/**
 * 获取冷料压库概况（D3）
 */
export async function getColdStockProfile(
  request: GetColdStockProfileRequest
): Promise<ColdStockProfileResponse> {
  return callWithValidation<ColdStockProfileResponse>(
    'get_cold_stock_profile',
    request,
    ColdStockProfileResponseSchema
  );
}

// ==========================================
// 简化API（使用默认参数）
// ==========================================

/**
 * 获取“哪天最危险”（兼容旧 dashboard_api.get_most_risky_date 语义）
 *
 * 默认:
 * - 日期范围: 今天 ~ 30 天后
 * - limit: 10
 * - sortBy: risk_score
 */
export async function getMostRiskyDate(
  versionId: string
): Promise<DecisionDaySummaryResponse> {
  const today = new Date();
  const dateFrom = new Date(today);
  const dateTo = new Date(today);
  dateTo.setDate(today.getDate() + 30);

  return getDecisionDaySummary({
    versionId,
    dateFrom: dateFrom.toISOString().split('T')[0],
    dateTo: dateTo.toISOString().split('T')[0],
    limit: 10,
    sortBy: 'risk_score',
  });
}

/**
 * 获取最近N天的风险摘要
 */
export async function getRiskSummaryForRecentDays(
  versionId: string,
  days: number = 30
): Promise<DecisionDaySummaryResponse> {
  const today = new Date();
  const dateFrom = new Date(today);
  dateFrom.setDate(today.getDate() - 1); // 从昨天开始
  const dateTo = new Date(today);
  dateTo.setDate(today.getDate() + days - 1); // 到未来N天

  return getDecisionDaySummary({
    versionId,
    dateFrom: dateFrom.toISOString().split('T')[0],
    dateTo: dateTo.toISOString().split('T')[0],
    limit: days,
  });
}

/**
 * 获取所有机组的堵塞概况（最近N天）
 */
export async function getBottleneckForRecentDays(
  versionId: string,
  days: number = 30
): Promise<MachineBottleneckProfileResponse> {
  const today = new Date();
  const dateFrom = new Date(today);
  dateFrom.setDate(today.getDate() - 1);
  const dateTo = new Date(today);
  dateTo.setDate(today.getDate() + days - 1);

  return getMachineBottleneckProfile({
    versionId,
    dateFrom: dateFrom.toISOString().split('T')[0],
    dateTo: dateTo.toISOString().split('T')[0],
  });
}

/**
 * 获取“哪个机组最堵”（兼容旧 dashboard_api.get_most_congested_machine 语义）
 *
 * 默认:
 * - 日期范围: 今天 ~ 7 天后
 * - limit: 10
 */
export async function getMostCongestedMachine(
  versionId: string
): Promise<MachineBottleneckProfileResponse> {
  const today = new Date();
  const dateFrom = new Date(today);
  const dateTo = new Date(today);
  dateTo.setDate(today.getDate() + 7);

  return getMachineBottleneckProfile({
    versionId,
    dateFrom: dateFrom.toISOString().split('T')[0],
    dateTo: dateTo.toISOString().split('T')[0],
    limit: 10,
  });
}

/**
 * 获取所有失败订单（不分页）
 */
export async function getAllFailedOrders(
  versionId: string
): Promise<OrderFailureSetResponse> {
  return listOrderFailureSet({
    versionId,
    limit: 1000, // 较大的限制，获取所有数据
  });
}

/**
 * 获取“哪些紧急单无法完成”（兼容旧 dashboard_api.get_unsatisfied_urgent_materials 语义）
 *
 * 默认:
 * - limit: 100
 */
export async function getUnsatisfiedUrgentMaterials(
  versionId: string
): Promise<OrderFailureSetResponse> {
  return listOrderFailureSet({
    versionId,
    limit: 100,
  });
}

/**
 * 获取“哪些冷料压库”（兼容旧 dashboard_api.get_cold_stock_materials 语义）
 *
 * 说明：旧接口的 thresholdDays 参数已在后端废弃，这里保留签名用于兼容，但不参与过滤。
 *
 * 默认:
 * - limit: 100
 */
export async function getColdStockMaterials(
  versionId: string,
  _thresholdDays?: number
): Promise<ColdStockProfileResponse> {
  return getColdStockProfile({
    versionId,
    limit: 100,
  });
}

/**
 * 获取高压力冷料（压力等级 >= HIGH）
 */
export async function getHighPressureColdStock(
  versionId: string
): Promise<ColdStockProfileResponse> {
  return getColdStockProfile({
    versionId,
    pressureLevelFilter: ['HIGH', 'CRITICAL'],
  });
}

// ==========================================
// D5: 轧制活动警报 API
// ==========================================

/**
 * 获取轧制活动警报（D5）
 */
export async function getRollCampaignAlert(
  request: GetRollCampaignAlertRequest
): Promise<RollCampaignAlertResponse> {
  return callWithValidation<RollCampaignAlertResponse>(
    'get_roll_campaign_alert',
    request,
    RollCampaignAlertResponseSchema
  );
}

/**
 * 获取所有机组的换辊警报（简化版）
 */
export async function getAllRollCampaignAlerts(
  versionId: string
): Promise<RollCampaignAlertResponse> {
  return getRollCampaignAlert({
    versionId,
  });
}

// ==========================================
// D6: 容量优化机会 API
// ==========================================

/**
 * 获取容量优化机会（D6）
 */
export async function getCapacityOpportunity(
  request: GetCapacityOpportunityRequest
): Promise<CapacityOpportunityResponse> {
  return callWithValidation<CapacityOpportunityResponse>(
    'get_capacity_opportunity',
    request,
    CapacityOpportunityResponseSchema
  );
}

/**
 * 获取最近N天的容量优化机会（简化版）
 */
export async function getCapacityOpportunityForRecentDays(
  versionId: string,
  days: number = 30
): Promise<CapacityOpportunityResponse> {
  const today = new Date();
  const dateFrom = new Date(today);
  dateFrom.setDate(today.getDate() - 1);
  const dateTo = new Date(today);
  dateTo.setDate(today.getDate() + days - 1);

  return getCapacityOpportunity({
    versionId,
    dateFrom: dateFrom.toISOString().split('T')[0],
    dateTo: dateTo.toISOString().split('T')[0],
    minOpportunityT: 10.0,
  });
}

// ==========================================
// 导出所有API
// ==========================================

export const decisionService = {
  // D1: 日期风险摘要
  getDecisionDaySummary,
  getMostRiskyDate,
  getRiskSummaryForRecentDays,

  // D4: 机组堵塞概况
  getMachineBottleneckProfile,
  getMostCongestedMachine,
  getBottleneckForRecentDays,

  // D2: 订单失败集合
  listOrderFailureSet,
  getUnsatisfiedUrgentMaterials,
  getAllFailedOrders,

  // D3: 冷料压库概况
  getColdStockProfile,
  getColdStockMaterials,
  getHighPressureColdStock,

  // D5: 轧制活动警报
  getRollCampaignAlert,
  getAllRollCampaignAlerts,

  // D6: 容量优化机会
  getCapacityOpportunity,
  getCapacityOpportunityForRecentDays,
};
