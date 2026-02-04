import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import PageSkeleton from './PageSkeleton';

describe('PageSkeleton 组件', () => {
  it('应该渲染骨架屏组件', () => {
    const { container } = render(<PageSkeleton />);

    const skeleton = container.querySelector('.ant-skeleton');
    expect(skeleton).toBeInTheDocument();
  });

  it('应该显示动画效果', () => {
    const { container } = render(<PageSkeleton />);

    const skeleton = container.querySelector('.ant-skeleton');
    expect(skeleton).toHaveClass('ant-skeleton-active');
  });

  it('应该有10行段落', () => {
    const { container } = render(<PageSkeleton />);

    const paragraphItems = container.querySelectorAll('.ant-skeleton-paragraph li');
    expect(paragraphItems.length).toBeGreaterThanOrEqual(10);
  });

  it('应该有标题', () => {
    const { container } = render(<PageSkeleton />);

    const title = container.querySelector('.ant-skeleton-title');
    expect(title).toBeInTheDocument();
  });

  it('应该有内边距样式', () => {
    const { container } = render(<PageSkeleton />);

    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper).toHaveStyle({ padding: '8px' });
  });
});
