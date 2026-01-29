import React from 'react';
import ErrorBoundary from '../components/ErrorBoundary';
import PageSkeleton from '../components/PageSkeleton';

const MaterialImport = React.lazy(() => import('../components/MaterialImport'));

const DataImport: React.FC = () => (
  <ErrorBoundary>
    <React.Suspense fallback={<PageSkeleton />}>
      <MaterialImport />
    </React.Suspense>
  </ErrorBoundary>
);

export default DataImport;

