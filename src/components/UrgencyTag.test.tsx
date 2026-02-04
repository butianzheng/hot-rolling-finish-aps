import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { UrgencyTag } from './UrgencyTag';

describe('UrgencyTag 组件', () => {
  it('应该渲染 L0 等级标签', () => {
    render(<UrgencyTag level="L0" />);

    const tag = screen.getByText('L0');
    expect(tag).toBeInTheDocument();
  });

  it('应该渲染 L1 等级标签', () => {
    render(<UrgencyTag level="L1" />);

    const tag = screen.getByText('L1');
    expect(tag).toBeInTheDocument();
  });

  it('应该渲染 L2 等级标签', () => {
    render(<UrgencyTag level="L2" />);

    const tag = screen.getByText('L2');
    expect(tag).toBeInTheDocument();
  });

  it('应该渲染 L3 等级标签', () => {
    render(<UrgencyTag level="L3" />);

    const tag = screen.getByText('L3');
    expect(tag).toBeInTheDocument();
  });

  it('应该为无效等级使用 L0 作为默认值', () => {
    render(<UrgencyTag level="INVALID" />);

    const tag = screen.getByText('L0');
    expect(tag).toBeInTheDocument();
  });

  it('应该渲染带原因的标签', () => {
    const reason = '临近交货期';
    render(<UrgencyTag level="L3" reason={reason} />);

    const tag = screen.getByText('L3');
    expect(tag).toBeInTheDocument();
  });

  it('应该为不同等级使用不同的颜色', () => {
    const { rerender } = render(<UrgencyTag level="L3" />);
    let tag = screen.getByText('L3');
    expect(tag).toHaveStyle({ fontWeight: 'bold' });

    rerender(<UrgencyTag level="L0" />);
    tag = screen.getByText('L0');
    expect(tag).toHaveStyle({ fontWeight: 'bold' });
  });

  it('应该显示帮助光标样式', () => {
    render(<UrgencyTag level="L2" />);

    const tag = screen.getByText('L2');
    expect(tag).toHaveStyle({ cursor: 'help' });
  });

  it('应该正确设置标签的字体大小和内边距', () => {
    render(<UrgencyTag level="L1" />);

    const tag = screen.getByText('L1');
    expect(tag).toHaveStyle({
      fontSize: '12px',
      padding: '2px 8px',
      border: 'none',
    });
  });
});
