import { useEffect, useRef, useState, useCallback } from 'react';

/**
 * 自动刷新 Hook - 用于定期执行刷新操作
 * @param callback - 刷新回调函数
 * @param interval - 刷新间隔（毫秒），默认 30000ms（30秒）
 * @param enabled - 是否启用自动刷新，默认 true
 * @returns {object} 包含最后刷新时间和手动刷新函数
 */
export function useAutoRefresh(
  callback: () => Promise<void> | void,
  interval: number = 30000,
  enabled: boolean = true
) {
  const [lastRefreshTime, setLastRefreshTime] = useState<Date | null>(null);
  const [nextRefreshCountdown, setNextRefreshCountdown] = useState<number>(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const countdownRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // 执行刷新
  const refresh = useCallback(async () => {
    try {
      await callback();
      setLastRefreshTime(new Date());
      // 重置倒计时
      setNextRefreshCountdown(Math.ceil(interval / 1000));
    } catch (error) {
      console.error('自动刷新失败:', error);
    }
  }, [callback, interval]);

  // 启动倒计时
  const startCountdown = useCallback(() => {
    if (countdownRef.current) {
      clearInterval(countdownRef.current);
    }

    setNextRefreshCountdown(Math.ceil(interval / 1000));

    countdownRef.current = setInterval(() => {
      setNextRefreshCountdown((prev) => {
        if (prev <= 1) {
          if (countdownRef.current) {
            clearInterval(countdownRef.current);
          }
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
  }, [interval]);

  // 设置自动刷新
  useEffect(() => {
    if (!enabled) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      if (countdownRef.current) {
        clearInterval(countdownRef.current);
      }
      return;
    }

    // 初始刷新
    refresh();
    startCountdown();

    // 设置定期刷新
    intervalRef.current = setInterval(() => {
      refresh();
      startCountdown();
    }, interval);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      if (countdownRef.current) {
        clearInterval(countdownRef.current);
      }
    };
  }, [enabled, interval, refresh, startCountdown]);

  return {
    lastRefreshTime,
    nextRefreshCountdown,
    refresh: () => {
      refresh();
      startCountdown();
    },
  };
}
