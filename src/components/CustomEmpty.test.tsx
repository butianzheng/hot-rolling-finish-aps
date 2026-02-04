import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import {
  CustomEmpty,
  tableEmptyConfig,
  tableSearchEmptyConfig,
  tableFilterEmptyConfig,
} from './CustomEmpty';

describe('CustomEmpty 组件', () => {
  it('应该渲染默认类型的空状态', () => {
    render(<CustomEmpty />);

    expect(screen.getByText('暂无数据')).toBeInTheDocument();
  });

  it('应该渲染数据类型的空状态', () => {
    render(<CustomEmpty type="data" />);

    expect(screen.getByText('暂无数据')).toBeInTheDocument();
  });

  it('应该渲染搜索类型的空状态', () => {
    render(<CustomEmpty type="search" />);

    expect(screen.getByText('未找到匹配的结果')).toBeInTheDocument();
  });

  it('应该渲染筛选类型的空状态', () => {
    render(<CustomEmpty type="filter" />);

    expect(screen.getByText('当前筛选条件下无数据')).toBeInTheDocument();
  });

  it('应该支持自定义描述', () => {
    render(<CustomEmpty type="data" description="自定义描述" />);

    expect(screen.getByText('自定义描述')).toBeInTheDocument();
  });

  it('应该传递额外的 props 给 Empty 组件', () => {
    const { container } = render(
      <CustomEmpty type="data" style={{ padding: '20px' }} />
    );

    const emptyElement = container.querySelector('.ant-empty');
    expect(emptyElement).toBeInTheDocument();
  });

  describe('表格空状态配置', () => {
    it('tableEmptyConfig 应该包含数据类型的空状态', () => {
      expect(tableEmptyConfig.emptyText).toBeDefined();
    });

    it('tableSearchEmptyConfig 应该包含搜索类型的空状态', () => {
      expect(tableSearchEmptyConfig.emptyText).toBeDefined();
    });

    it('tableFilterEmptyConfig 应该包含筛选类型的空状态', () => {
      expect(tableFilterEmptyConfig.emptyText).toBeDefined();
    });
  });
});
