export type BuiltinStrategyType =
  | 'balanced'
  | 'urgent_first'
  | 'capacity_first'
  | 'cold_stock_first'
  | 'manual';

export type StrategyType = BuiltinStrategyType | `custom:${string}`;

export interface UserPreferences {
  defaultTheme: 'light' | 'dark';
  autoRefreshInterval: number;
  sidebarCollapsed: boolean;
  defaultStrategy: StrategyType;
}
