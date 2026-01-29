import React from 'react';
import { Skeleton } from 'antd';

const PageSkeleton: React.FC = () => (
  <div style={{ padding: 8 }}>
    <Skeleton active paragraph={{ rows: 10 }} />
  </div>
);

export default React.memo(PageSkeleton);

