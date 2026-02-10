import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeToggle } from './ThemeToggle';

// Mock theme and stores
const mockTheme = vi.fn();
const mockToggleTheme = vi.fn();
const mockUpdateUserPreferences = vi.fn();

vi.mock('../theme', () => ({
  useTheme: () => ({ theme: mockTheme(), toggleTheme: mockToggleTheme }),
}));

vi.mock('../stores/use-global-store', () => ({
  useGlobalActions: () => ({ updateUserPreferences: mockUpdateUserPreferences }),
}));

describe('ThemeToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('应该在暗色模式下渲染正确的图标', () => {
    mockTheme.mockReturnValue('dark');

    const { container } = render(<ThemeToggle />);

    // 暗色模式应该显示 BulbOutlined
    const icon = container.querySelector('.anticon-bulb');
    expect(icon).toBeInTheDocument();
  });

  it('应该在亮色模式下渲染正确的图标', () => {
    mockTheme.mockReturnValue('light');

    const { container } = render(<ThemeToggle />);

    // 亮色模式应该显示 BulbFilled
    const icon = container.querySelector('.anticon-bulb');
    expect(icon).toBeInTheDocument();
  });

  it('应该在点击时切换主题', async () => {
    const user = userEvent.setup();
    mockTheme.mockReturnValue('dark');

    render(<ThemeToggle />);

    const button = screen.getByRole('button');
    await user.click(button);

    expect(mockToggleTheme).toHaveBeenCalled();
    expect(mockUpdateUserPreferences).toHaveBeenCalledWith({ defaultTheme: 'light' });
  });

  it('应该从亮色切换到暗色', async () => {
    const user = userEvent.setup();
    mockTheme.mockReturnValue('light');

    render(<ThemeToggle />);

    const button = screen.getByRole('button');
    await user.click(button);

    expect(mockToggleTheme).toHaveBeenCalled();
    expect(mockUpdateUserPreferences).toHaveBeenCalledWith({ defaultTheme: 'dark' });
  });
});
