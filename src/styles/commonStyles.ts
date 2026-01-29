import type React from 'react';
import { BORDER_RADIUS, SPACING } from '../theme/tokens';

export const containerStyles: Record<string, React.CSSProperties> = {
  card: {
    marginBottom: SPACING.BASE,
    borderRadius: BORDER_RADIUS.LG,
    padding: SPACING.BASE,
  },
  flexCenter: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
};

