import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RedLineGuard } from './RedLineGuard';
import type { RedLineViolation } from '../red-line-guard/types';

describe('RedLineGuard 组件', () => {
  const mockViolations: RedLineViolation[] = [
    {
      type: 'FROZEN_ZONE_PROTECTION',
      message: '该材料已锁定，不可调整',
      severity: 'error',
    },
  ];

  it('应该在无违规时不渲染', () => {
    const { container } = render(<RedLineGuard violations={[]} />);

    expect(container.firstChild).toBeNull();
  });

  it('应该在无 violations 时不渲染', () => {
    const { container } = render(<RedLineGuard violations={undefined as any} />);

    expect(container.firstChild).toBeNull();
  });

  it('应该在紧凑模式下渲染', () => {
    render(<RedLineGuard violations={mockViolations} mode="compact" />);

    expect(screen.getByText('冻结区保护')).toBeInTheDocument();
  });

  it('应该默认使用紧凑模式', () => {
    render(<RedLineGuard violations={mockViolations} />);

    expect(screen.getByText('冻结区保护')).toBeInTheDocument();
  });

  it('应该在详细模式下渲染', () => {
    render(<RedLineGuard violations={mockViolations} mode="detailed" />);

    expect(screen.getByText('该材料已锁定，不可调整')).toBeInTheDocument();
  });

  it('应该在详细模式下显示 Alert 组件', () => {
    const { container } = render(
      <RedLineGuard violations={mockViolations} mode="detailed" />
    );

    const alert = container.querySelector('.ant-alert');
    expect(alert).toBeInTheDocument();
  });

  it('应该支持多个违规', () => {
    const violations: RedLineViolation[] = [
      {
        type: 'FROZEN_ZONE_PROTECTION',
        message: '该材料已锁定',
        severity: 'error',
      },
      {
        type: 'MATURITY_CONSTRAINT',
        message: '材料未适温',
        severity: 'warning',
      },
    ];

    render(<RedLineGuard violations={violations} mode="detailed" />);

    expect(screen.getByText('该材料已锁定')).toBeInTheDocument();
    expect(screen.getByText('材料未适温')).toBeInTheDocument();
  });

  it('应该在详细模式下显示违规详情', () => {
    const violations: RedLineViolation[] = [
      {
        type: 'MATURITY_CONSTRAINT',
        message: '材料未适温',
        severity: 'warning',
        details: '距离适温还需2天',
      },
    ];

    render(<RedLineGuard violations={violations} mode="detailed" />);

    expect(screen.getByText('距离适温还需2天')).toBeInTheDocument();
  });

  it('应该在详细模式下显示受影响的实体', () => {
    const violations: RedLineViolation[] = [
      {
        type: 'MATURITY_CONSTRAINT',
        message: '材料未适温',
        severity: 'warning',
        affectedEntities: ['M12345678', 'M87654321'],
      },
    ];

    render(<RedLineGuard violations={violations} mode="detailed" />);

    expect(screen.getByText(/M12345678/)).toBeInTheDocument();
    expect(screen.getByText(/M87654321/)).toBeInTheDocument();
  });

  it('应该支持 closable 属性', () => {
    render(
      <RedLineGuard
        violations={mockViolations}
        mode="detailed"
        closable={true}
      />
    );

    const closeButton = screen.getByRole('button', { name: /close/i });
    expect(closeButton).toBeInTheDocument();
  });

  it('应该在点击关闭按钮时调用 onClose', async () => {
    const onClose = vi.fn();

    render(
      <RedLineGuard
        violations={mockViolations}
        mode="detailed"
        closable={true}
        onClose={onClose}
      />
    );

    const closeButton = screen.getByRole('button', { name: /close/i });
    await userEvent.click(closeButton);

    expect(onClose).toHaveBeenCalled();
  });

  it('应该根据 severity 使用不同的样式', () => {
    const errorViolations: RedLineViolation[] = [
      {
        type: 'FROZEN_ZONE_PROTECTION',
        message: '错误级别违规',
        severity: 'error',
      },
    ];

    const warningViolations: RedLineViolation[] = [
      {
        type: 'MATURITY_CONSTRAINT',
        message: '警告级别违规',
        severity: 'warning',
      },
    ];

    const { rerender } = render(
      <RedLineGuard violations={errorViolations} mode="detailed" />
    );
    expect(screen.getByText('错误级别违规')).toBeInTheDocument();

    rerender(<RedLineGuard violations={warningViolations} mode="detailed" />);
    expect(screen.getByText('警告级别违规')).toBeInTheDocument();
  });

  it('应该显示所有红线类型的违规', () => {
    const allViolations: RedLineViolation[] = [
      {
        type: 'FROZEN_ZONE_PROTECTION',
        message: '冻结区保护违规',
        severity: 'error',
      },
      {
        type: 'MATURITY_CONSTRAINT',
        message: '适温约束违规',
        severity: 'warning',
      },
      {
        type: 'LAYERED_URGENCY',
        message: '分层紧急度违规',
        severity: 'warning',
      },
      {
        type: 'CAPACITY_FIRST',
        message: '容量优先违规',
        severity: 'error',
      },
      {
        type: 'EXPLAINABILITY',
        message: '可解释性违规',
        severity: 'warning',
      },
    ];

    render(<RedLineGuard violations={allViolations} mode="detailed" />);

    expect(screen.getByText('冻结区保护违规')).toBeInTheDocument();
    expect(screen.getByText('适温约束违规')).toBeInTheDocument();
    expect(screen.getByText('分层紧急度违规')).toBeInTheDocument();
    expect(screen.getByText('容量优先违规')).toBeInTheDocument();
    expect(screen.getByText('可解释性违规')).toBeInTheDocument();
  });
});
