import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AdminOverrideToggle } from './AdminOverrideToggle';

// Mock stores
const mockUseAdminOverrideMode = vi.fn();
const mockSetAdminOverrideMode = vi.fn();

vi.mock('../stores/use-global-store', () => ({
  useAdminOverrideMode: () => mockUseAdminOverrideMode(),
  useGlobalActions: () => ({ setAdminOverrideMode: mockSetAdminOverrideMode }),
}));

describe('AdminOverrideToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('应该渲染关闭状态的开关', () => {
    mockUseAdminOverrideMode.mockReturnValue(false);

    render(<AdminOverrideToggle />);

    const switchElement = screen.getByRole('switch');
    expect(switchElement).toBeInTheDocument();
    expect(switchElement).not.toBeChecked();
  });

  it('应该渲染开启状态的开关', () => {
    mockUseAdminOverrideMode.mockReturnValue(true);

    render(<AdminOverrideToggle />);

    const switchElement = screen.getByRole('switch');
    expect(switchElement).toBeChecked();
  });

  it('应该在开启时显示警告图标', () => {
    mockUseAdminOverrideMode.mockReturnValue(true);

    const { container } = render(<AdminOverrideToggle />);

    // 查找警告图标
    const warningIcon = container.querySelector('.anticon-warning');
    expect(warningIcon).toBeInTheDocument();
  });

  it('应该在关闭时不显示警告图标', () => {
    mockUseAdminOverrideMode.mockReturnValue(false);

    const { container } = render(<AdminOverrideToggle />);

    // 警告图标不应该存在
    const warningIcon = container.querySelector('.anticon-warning');
    expect(warningIcon).not.toBeInTheDocument();
  });

  it('应该在点击时调用 setAdminOverrideMode', async () => {
    const user = userEvent.setup();
    mockUseAdminOverrideMode.mockReturnValue(false);

    render(<AdminOverrideToggle />);

    const switchElement = screen.getByRole('switch');
    await user.click(switchElement);

    // Ant Design Switch 的 onChange 会传递 checked 和 event 两个参数
    expect(mockSetAdminOverrideMode).toHaveBeenCalled();
    const firstCall = mockSetAdminOverrideMode.mock.calls[0];
    expect(firstCall[0]).toBe(true); // 第一个参数是 checked 状态
  });
});
