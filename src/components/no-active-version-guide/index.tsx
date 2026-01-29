/**
 * NoActiveVersionGuide - 主组件
 *
 * 重构后：204 行 → ~40 行 (-80%)
 */

import React from 'react';
import { Col, Row } from 'antd';
import type { NoActiveVersionGuideProps } from './types';
import { MainHintCard } from './MainHintCard';
import { QuickStartGuide } from './QuickStartGuide';
import { FAQCard } from './FAQCard';

const NoActiveVersionGuide: React.FC<NoActiveVersionGuideProps> = ({
  onNavigateToPlan,
  onNavigateToImport,
  title = '尚无激活的排产版本',
  description = '请先创建并激活一个排产版本，才能进行排产和调度操作',
}) => {
  return (
    <div style={{ padding: '40px 20px' }}>
      <Row justify="center" gutter={[24, 24]}>
        <Col xs={24} sm={22} md={20} lg={16} xl={14}>
          {/* 主要提示区域 */}
          <MainHintCard
            title={title}
            description={description}
            onNavigateToPlan={onNavigateToPlan}
            onNavigateToImport={onNavigateToImport}
          />

          {/* 操作步骤指南 */}
          <QuickStartGuide showImportStep={!!onNavigateToImport} />

          {/* 常见问题 */}
          <FAQCard />
        </Col>
      </Row>
    </div>
  );
};

export default NoActiveVersionGuide;
