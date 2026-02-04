import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ErrorBoundary from './ErrorBoundary';
import * as telemetry from '../utils/telemetry';

// Mock 遥测模块
vi.mock('../utils/telemetry', () => ({
  reportFrontendError: vi.fn(),
}));

// 创建一个会抛出错误的组件
const ThrowError = ({ shouldThrow }: { shouldThrow: boolean }) => {
  if (shouldThrow) {
    throw new Error('测试错误');
  }
  return <div>正常内容</div>;
};

// 用于抑制 console.error 在测试中的输出
const consoleError = console.error;

describe('ErrorBoundary 组件', () => {
  beforeEach(() => {
    // 抑制 React 错误边界的 console.error
    console.error = vi.fn();
  });

  afterEach(() => {
    console.error = consoleError;
    vi.restoreAllMocks();
  });

  it('应该渲染正常的子组件', () => {
    render(
      <ErrorBoundary>
        <div>正常内容</div>
      </ErrorBoundary>
    );

    expect(screen.getByText('正常内容')).toBeInTheDocument();
  });

  it('应该捕获子组件错误并显示错误UI', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('页面渲染错误')).toBeInTheDocument();
    expect(screen.getByText(/抱歉，页面遇到了渲染错误/)).toBeInTheDocument();
  });

  it('应该显示刷新页面按钮', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    const refreshButton = screen.getByRole('button', { name: '刷新页面' });
    expect(refreshButton).toBeInTheDocument();
  });

  it('应该显示返回上一页按钮', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    const backButton = screen.getByRole('button', { name: '返回上一页' });
    expect(backButton).toBeInTheDocument();
  });

  it('应该显示查看错误详情按钮', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('查看错误详情')).toBeInTheDocument();
  });

  it('应该显示错误消息', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    const errorDetails = screen.getByText(/Error: 测试错误/);
    expect(errorDetails).toBeInTheDocument();
  });

  it('点击刷新页面按钮应该重新加载页面', async () => {
    const reloadMock = vi.fn();
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { reload: reloadMock },
    });

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    const refreshButton = screen.getByRole('button', { name: '刷新页面' });
    await userEvent.click(refreshButton);

    expect(reloadMock).toHaveBeenCalled();
  });

  it('点击返回上一页按钮应该返回上一页', async () => {
    const backMock = vi.fn();
    Object.defineProperty(window.history, 'back', {
      writable: true,
      value: backMock,
    });

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    const backButton = screen.getByRole('button', { name: '返回上一页' });
    await userEvent.click(backButton);

    expect(backMock).toHaveBeenCalled();
  });

  it('应该在捕获错误时调用 reportFrontendError', () => {
    const reportSpy = vi.spyOn(telemetry, 'reportFrontendError');

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(reportSpy).toHaveBeenCalled();
    expect(reportSpy).toHaveBeenCalledWith(
      expect.any(Error),
      expect.objectContaining({
        source: 'ErrorBoundary',
      })
    );
  });

  it('应该在错误发生后不渲染子组件', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.queryByText('正常内容')).not.toBeInTheDocument();
  });

  it('应该显示错误状态的 Result 组件', () => {
    const { container } = render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    const result = container.querySelector('.ant-result-error');
    expect(result).toBeInTheDocument();
  });
});
