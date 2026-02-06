/**
 * 基本信息区组件
 */

import React from 'react';
import { Descriptions, Typography } from 'antd';
import { FONT_FAMILIES } from '../../theme';
import type { Material } from './types';
import { formatWeight } from '../../utils/formatters';

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
          {material.weight_t != null ? formatWeight(material.weight_t) : '-'}
        </Text>
      </Descriptions.Item>
      <Descriptions.Item label="钢种">
        {material.steel_mark || '-'}
      </Descriptions.Item>
    </Descriptions>
  );
};

export default BasicInfoSection;
