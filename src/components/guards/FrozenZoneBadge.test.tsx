import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FrozenZoneBadge } from './FrozenZoneBadge';

describe('FrozenZoneBadge 组件', () => {
  it('应该不渲染未锁定的徽章', () => {
    const { container } = render(<FrozenZoneBadge locked={false} />);

    expect(container.firstChild).toBeNull();
  });

  it('应该渲染锁定的徽章（badge 模式）', () => {
    render(<FrozenZoneBadge locked={true} />);

    expect(screen.getByText('冻结区')).toBeInTheDocument();
  });

  it('应该在 badge 模式下显示锁定图标', () => {
    const { container } = render(<FrozenZoneBadge locked={true} mode="badge" />);

    const icon = container.querySelector('.anticon-lock');
    expect(icon).toBeInTheDocument();
  });

  it('应该在 badge 模式下使用红色标签', () => {
    render(<FrozenZoneBadge locked={true} mode="badge" />);

    const tag = screen.getByText('冻结区');
    expect(tag.parentElement).toBeInTheDocument();
  });

  it('应该在 banner 模式下渲染横幅', () => {
    render(<FrozenZoneBadge locked={true} mode="banner" />);

    expect(screen.getByText('冻结区保护（工业红线1）')).toBeInTheDocument();
  });

  it('应该在 banner 模式下显示默认提示', () => {
    render(<FrozenZoneBadge locked={true} mode="banner" />);

    expect(
      screen.getByText('冻结材料不可自动调整（红线保护）')
    ).toBeInTheDocument();
  });

  it('应该在 banner 模式下显示自定义锁定原因', () => {
    const lockReason = '已进入冻结区，不可调整';
    render(
      <FrozenZoneBadge locked={true} mode="banner" lockReason={lockReason} />
    );

    expect(screen.getByText(lockReason)).toBeInTheDocument();
  });

  it('应该在 banner 模式下显示锁定图标', () => {
    const { container } = render(
      <FrozenZoneBadge locked={true} mode="banner" />
    );

    const icon = container.querySelector('.anticon-lock');
    expect(icon).toBeInTheDocument();
  });

  it('应该支持自定义 tooltip 标题', () => {
    const customTitle = '自定义提示信息';
    render(
      <FrozenZoneBadge
        locked={true}
        mode="badge"
        tooltipTitle={customTitle}
      />
    );

    expect(screen.getByText('冻结区')).toBeInTheDocument();
  });

  it('应该在 badge 模式下优先显示 lockReason', () => {
    const lockReason = '已冻结';
    render(
      <FrozenZoneBadge
        locked={true}
        mode="badge"
        tooltipTitle="默认提示"
        lockReason={lockReason}
      />
    );

    expect(screen.getByText('冻结区')).toBeInTheDocument();
  });

  it('应该在 banner 模式下有正确的样式', () => {
    const { container } = render(
      <FrozenZoneBadge locked={true} mode="banner" />
    );

    const banner = container.firstChild as HTMLElement;
    expect(banner).toHaveStyle({
      background: '#fff1f0',
      borderRadius: '4px',
      padding: '8px 16px',
    });
  });

  it('应该在未锁定时返回 null', () => {
    const { container } = render(<FrozenZoneBadge locked={false} />);

    expect(container.firstChild).toBeNull();
  });

  it('应该默认使用 badge 模式', () => {
    render(<FrozenZoneBadge locked={true} />);

    expect(screen.getByText('冻结区')).toBeInTheDocument();
    expect(screen.queryByText('冻结区保护（工业红线1）')).not.toBeInTheDocument();
  });
});
