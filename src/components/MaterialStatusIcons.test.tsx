import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { MaterialStatusIcons } from './MaterialStatusIcons';

describe('MaterialStatusIcons 组件', () => {
  it('应该在锁定时显示锁定图标', () => {
    const { container } = render(
      <MaterialStatusIcons lockFlag={true} schedState="PENDING" />
    );

    const lockIcon = container.querySelector('.anticon-lock');
    expect(lockIcon).toBeInTheDocument();
  });

  it('应该在未锁定时不显示锁定图标', () => {
    const { container } = render(
      <MaterialStatusIcons lockFlag={false} schedState="PENDING" />
    );

    const lockIcon = container.querySelector('.anticon-lock');
    expect(lockIcon).not.toBeInTheDocument();
  });

  it('应该在有温度问题时显示火焰图标', () => {
    const { container } = render(
      <MaterialStatusIcons
        lockFlag={false}
        schedState="PENDING"
        tempIssue={true}
      />
    );

    const fireIcon = container.querySelector('.anticon-fire');
    expect(fireIcon).toBeInTheDocument();
  });

  it('应该在无温度问题时不显示火焰图标', () => {
    const { container } = render(
      <MaterialStatusIcons
        lockFlag={false}
        schedState="PENDING"
        tempIssue={false}
      />
    );

    const fireIcon = container.querySelector('.anticon-fire');
    expect(fireIcon).not.toBeInTheDocument();
  });

  it('应该在已排产时显示勾选图标', () => {
    const { container } = render(
      <MaterialStatusIcons lockFlag={false} schedState="SCHEDULED" />
    );

    const checkIcon = container.querySelector('.anticon-check-circle');
    expect(checkIcon).toBeInTheDocument();
  });

  it('应该在未排产时不显示勾选图标', () => {
    const { container } = render(
      <MaterialStatusIcons lockFlag={false} schedState="PENDING" />
    );

    const checkIcon = container.querySelector('.anticon-check-circle');
    expect(checkIcon).not.toBeInTheDocument();
  });

  it('应该同时显示多个状态图标', () => {
    const { container } = render(
      <MaterialStatusIcons
        lockFlag={true}
        schedState="SCHEDULED"
        tempIssue={true}
      />
    );

    const lockIcon = container.querySelector('.anticon-lock');
    const fireIcon = container.querySelector('.anticon-fire');
    const checkIcon = container.querySelector('.anticon-check-circle');

    expect(lockIcon).toBeInTheDocument();
    expect(fireIcon).toBeInTheDocument();
    expect(checkIcon).toBeInTheDocument();
  });

  it('应该在无任何状态时不显示图标', () => {
    const { container } = render(
      <MaterialStatusIcons
        lockFlag={false}
        schedState="PENDING"
        tempIssue={false}
      />
    );

    const lockIcon = container.querySelector('.anticon-lock');
    const fireIcon = container.querySelector('.anticon-fire');
    const checkIcon = container.querySelector('.anticon-check-circle');

    expect(lockIcon).not.toBeInTheDocument();
    expect(fireIcon).not.toBeInTheDocument();
    expect(checkIcon).not.toBeInTheDocument();
  });

  it('应该使用 Space 组件包裹图标', () => {
    const { container } = render(
      <MaterialStatusIcons lockFlag={true} schedState="PENDING" />
    );

    const space = container.querySelector('.ant-space');
    expect(space).toBeInTheDocument();
  });

  it('应该默认 tempIssue 为 false', () => {
    const { container } = render(
      <MaterialStatusIcons lockFlag={false} schedState="PENDING" />
    );

    const fireIcon = container.querySelector('.anticon-fire');
    expect(fireIcon).not.toBeInTheDocument();
  });
});
