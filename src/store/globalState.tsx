import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';

const STORAGE_KEY = 'aps_current_user';

interface GlobalState {
  activeVersionId: string | null;
  isRecalculating: boolean;
  isImporting: boolean;
  currentUser: string;
  adminOverrideMode: boolean;
}

interface GlobalStateContextType extends GlobalState {
  setActiveVersion: (id: string | null) => void;
  setRecalculating: (flag: boolean) => void;
  setImporting: (flag: boolean) => void;
  setCurrentUser: (user: string) => void;
  setAdminOverrideMode: (flag: boolean) => void;
}

const GlobalStateContext = createContext<GlobalStateContextType | undefined>(undefined);

export const GlobalStateProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [state, setState] = useState<GlobalState>(() => {
    // 从 localStorage 读取用户信息
    const savedUser = localStorage.getItem(STORAGE_KEY);
    return {
      activeVersionId: null,
      isRecalculating: false,
      isImporting: false,
      currentUser: savedUser || 'admin',
      adminOverrideMode: false,
    };
  });

  // 持久化用户信息
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, state.currentUser);
  }, [state.currentUser]);

  const setActiveVersion = (id: string | null) => {
    setState(prev => ({ ...prev, activeVersionId: id }));
  };

  const setRecalculating = (flag: boolean) => {
    setState(prev => ({ ...prev, isRecalculating: flag }));
  };

  const setImporting = (flag: boolean) => {
    setState(prev => ({ ...prev, isImporting: flag }));
  };

  const setCurrentUser = (user: string) => {
    setState(prev => ({ ...prev, currentUser: user }));
  };

  const setAdminOverrideMode = (flag: boolean) => {
    setState(prev => ({ ...prev, adminOverrideMode: flag }));
  };

  return (
    <GlobalStateContext.Provider
      value={{
        ...state,
        setActiveVersion,
        setRecalculating,
        setImporting,
        setCurrentUser,
        setAdminOverrideMode,
      }}
    >
      {children}
    </GlobalStateContext.Provider>
  );
};

export const useGlobalState = () => {
  const context = useContext(GlobalStateContext);
  if (!context) {
    throw new Error('useGlobalState must be used within GlobalStateProvider');
  }
  return context;
};
