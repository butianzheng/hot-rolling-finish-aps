// ==========================================
// 性能监控 Hook
// ==========================================
// 职责: 监控组件渲染性能，识别性能瓶颈
// ==========================================

import { useEffect, useRef, useState } from 'react';

// ==========================================
// 类型定义
// ==========================================

/**
 * 性能指标
 */
export interface PerformanceMetrics {
  /** 组件名称 */
  componentName: string;
  /** 渲染次数 */
  renderCount: number;
  /** 平均渲染时间（毫秒） */
  avgRenderTime: number;
  /** 最大渲染时间（毫秒） */
  maxRenderTime: number;
  /** 最小渲染时间（毫秒） */
  minRenderTime: number;
  /** 最近一次渲染时间（毫秒） */
  lastRenderTime: number;
  /** 是否存在性能问题 */
  hasPerformanceIssue: boolean;
}

/**
 * 性能监控配置
 */
export interface PerformanceMonitorConfig {
  /** 组件名称 */
  componentName: string;
  /** 性能警告阈值（毫秒） */
  warningThreshold?: number;
  /** 是否启用日志 */
  enableLogging?: boolean;
  /** 最大样本数 */
  maxSamples?: number;
}

// ==========================================
// 常量
// ==========================================

const DEFAULT_WARNING_THRESHOLD = 16; // 60fps = 16.67ms/frame
const DEFAULT_MAX_SAMPLES = 20;

// ==========================================
// 主Hook
// ==========================================

/**
 * 性能监控Hook
 *
 * 监控组件渲染性能，记录渲染时间和次数
 *
 * @param config - 监控配置
 * @returns 性能指标
 *
 * @example
 * ```tsx
 * const MyComponent = () => {
 *   const metrics = usePerformanceMonitor({
 *     componentName: 'MyComponent',
 *     warningThreshold: 16,
 *     enableLogging: true,
 *   });
 *
 *   // 组件逻辑...
 *
 *   return <div>Render count: {metrics.renderCount}</div>;
 * };
 * ```
 */
export function usePerformanceMonitor(
  config: PerformanceMonitorConfig
): PerformanceMetrics {
  const {
    componentName,
    warningThreshold = DEFAULT_WARNING_THRESHOLD,
    enableLogging = false,
    maxSamples = DEFAULT_MAX_SAMPLES,
  } = config;

  // 渲染计数
  const renderCountRef = useRef(0);
  // 渲染时间记录
  const renderTimesRef = useRef<number[]>([]);
  // 开始时间
  const startTimeRef = useRef<number>(0);

  // 性能指标状态
  const [metrics, setMetrics] = useState<PerformanceMetrics>({
    componentName,
    renderCount: 0,
    avgRenderTime: 0,
    maxRenderTime: 0,
    minRenderTime: 0,
    lastRenderTime: 0,
    hasPerformanceIssue: false,
  });

  // 记录渲染开始时间
  startTimeRef.current = performance.now();

  useEffect(() => {
    // 计算渲染时间
    const renderTime = performance.now() - startTimeRef.current;

    // 更新渲染计数
    renderCountRef.current += 1;

    // 记录渲染时间
    renderTimesRef.current.push(renderTime);
    if (renderTimesRef.current.length > maxSamples) {
      renderTimesRef.current.shift();
    }

    // 计算统计数据
    const times = renderTimesRef.current;
    const avgRenderTime = times.reduce((a, b) => a + b, 0) / times.length;
    const maxRenderTime = Math.max(...times);
    const minRenderTime = Math.min(...times);

    // 判断是否存在性能问题
    const hasPerformanceIssue = avgRenderTime > warningThreshold;

    // 更新指标
    setMetrics({
      componentName,
      renderCount: renderCountRef.current,
      avgRenderTime,
      maxRenderTime,
      minRenderTime,
      lastRenderTime: renderTime,
      hasPerformanceIssue,
    });

    // 日志输出
    if (enableLogging) {
      console.log(
        `[Performance] ${componentName} #${renderCountRef.current}:`,
        `${renderTime.toFixed(2)}ms`,
        hasPerformanceIssue ? '⚠️ SLOW' : '✅'
      );

      if (hasPerformanceIssue) {
        console.warn(
          `[Performance Warning] ${componentName} average render time (${avgRenderTime.toFixed(2)}ms) exceeds threshold (${warningThreshold}ms)`
        );
      }
    }
  });

  return metrics;
}

// ==========================================
// 辅助Hook：渲染次数监控
// ==========================================

/**
 * 监控组件渲染次数
 *
 * @param componentName - 组件名称
 * @param enableLogging - 是否启用日志
 * @returns 渲染次数
 *
 * @example
 * ```tsx
 * const MyComponent = () => {
 *   const renderCount = useRenderCount('MyComponent', true);
 *   return <div>Rendered {renderCount} times</div>;
 * };
 * ```
 */
export function useRenderCount(
  componentName: string,
  enableLogging: boolean = false
): number {
  const renderCountRef = useRef(0);

  renderCountRef.current += 1;

  useEffect(() => {
    if (enableLogging) {
      console.log(`[Render] ${componentName} rendered ${renderCountRef.current} times`);
    }
  });

  return renderCountRef.current;
}

// ==========================================
// 辅助Hook：Props变化监控
// ==========================================

/**
 * 监控Props变化
 *
 * 用于识别导致重渲染的props变化
 *
 * @param componentName - 组件名称
 * @param props - 组件props
 * @param enableLogging - 是否启用日志
 *
 * @example
 * ```tsx
 * const MyComponent = (props) => {
 *   usePropsChangeMonitor('MyComponent', props, true);
 *   // 组件逻辑...
 * };
 * ```
 */
export function usePropsChangeMonitor<T extends Record<string, any>>(
  componentName: string,
  props: T,
  enableLogging: boolean = false
): void {
  const prevPropsRef = useRef<T>(props);

  useEffect(() => {
    if (!enableLogging) return;

    const changedProps: string[] = [];
    const prevProps = prevPropsRef.current;

    // 比较props
    Object.keys(props).forEach((key) => {
      if (props[key] !== prevProps[key]) {
        changedProps.push(key);
      }
    });

    if (changedProps.length > 0) {
      console.log(
        `[Props Change] ${componentName} re-rendered due to:`,
        changedProps.join(', ')
      );
      console.log('  变更前：', changedProps.map((k) => `${k}=${prevProps[k]}`));
      console.log('  变更后：', changedProps.map((k) => `${k}=${props[k]}`));
    }

    prevPropsRef.current = props;
  });
}

// ==========================================
// 辅助Hook：慢渲染检测
// ==========================================

/**
 * 慢渲染检测Hook
 *
 * 当渲染时间超过阈值时触发回调
 *
 * @param componentName - 组件名称
 * @param threshold - 阈值（毫秒）
 * @param onSlowRender - 慢渲染回调
 *
 * @example
 * ```tsx
 * const MyComponent = () => {
 *   useSlowRenderDetector('MyComponent', 16, (renderTime) => {
 *     console.warn(`Slow render detected: ${renderTime}ms`);
 *   });
 *   // 组件逻辑...
 * };
 * ```
 */
export function useSlowRenderDetector(
  componentName: string,
  threshold: number = DEFAULT_WARNING_THRESHOLD,
  onSlowRender?: (renderTime: number) => void
): void {
  const startTimeRef = useRef<number>(0);
  startTimeRef.current = performance.now();

  useEffect(() => {
    const renderTime = performance.now() - startTimeRef.current;

    if (renderTime > threshold) {
      console.warn(
        `[Slow Render] ${componentName} took ${renderTime.toFixed(2)}ms (threshold: ${threshold}ms)`
      );

      if (onSlowRender) {
        onSlowRender(renderTime);
      }
    }
  });
}

// ==========================================
// 工具函数
// ==========================================

/**
 * 格式化性能指标为可读字符串
 */
export function formatMetrics(metrics: PerformanceMetrics): string {
  return `
Component: ${metrics.componentName}
Render Count: ${metrics.renderCount}
Avg Render Time: ${metrics.avgRenderTime.toFixed(2)}ms
Max Render Time: ${metrics.maxRenderTime.toFixed(2)}ms
Min Render Time: ${metrics.minRenderTime.toFixed(2)}ms
Last Render Time: ${metrics.lastRenderTime.toFixed(2)}ms
Performance Issue: ${metrics.hasPerformanceIssue ? '⚠️ YES' : '✅ NO'}
  `.trim();
}

/**
 * 将性能指标导出为CSV格式
 */
export function metricsToCSV(metricsList: PerformanceMetrics[]): string {
  const headers = [
    'Component Name',
    'Render Count',
    'Avg Render Time (ms)',
    'Max Render Time (ms)',
    'Min Render Time (ms)',
    'Last Render Time (ms)',
    'Has Performance Issue',
  ];

  const rows = metricsList.map((m) => [
    m.componentName,
    m.renderCount.toString(),
    m.avgRenderTime.toFixed(2),
    m.maxRenderTime.toFixed(2),
    m.minRenderTime.toFixed(2),
    m.lastRenderTime.toFixed(2),
    m.hasPerformanceIssue ? 'YES' : 'NO',
  ]);

  return [headers.join(','), ...rows.map((r) => r.join(','))].join('\n');
}

// ==========================================
// 默认导出
// ==========================================

export default usePerformanceMonitor;
