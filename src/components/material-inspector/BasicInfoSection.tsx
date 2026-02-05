/**
 * 基本信息区组件
 */

import React from 'react';
import { Descriptions, Typography } from 'antd';
import { FONT_FAMILIES } from '../../theme';
import type { Material } from './types';

const { Text } = Typography;

export interface BasicInfoSectionProps {
  material: Material;
}

export const BasicInfoSection: React.FC<BasicInfoSectionProps> = ({ material }) => {
  return (
    <Descriptions column={1} size="small" bordered>
      <Descriptions.Item label="材料号">
        <Text
          copyable
          style={{ fontFamily: FONT_FAMILIES.MONOSPACE, fontSize: 13 }}
        >
          {material.material_id}
        </Text>
      </Descriptions.Item>
      <Descriptions.Item label="机组">
        {material.machine_code || '-'}
      </Descriptions.Item>
      <Descriptions.Item label="重量">
        <Text style={{ fontFamily: FONT_FAMILIES.MONOSPACE }}>
          {material.weight_t ? `${material.weight_t.toFixed(3)} 吨` : '-'}
        </Text>
      </Descriptions.Item>
      <Descriptions.Item label="钢种">
        {material.steel_mark || '-'}
      </Descriptions.Item>
    </Descriptions>
  );
};

export default BasicInfoSection;
