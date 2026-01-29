export type StrategyType =
  | 'balanced'
  | 'urgent_first'
  | 'capacity_first'
  | 'cold_stock_first'
  | 'manual';

export interface UserPreferences {
  defaultTheme: 'light' | 'dark';
  autoRefreshInterval: number;
  sidebarCollapsed: boolean;
  defaultStrategy: StrategyType;
}

