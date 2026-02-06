import React, { Component, ReactNode } from 'react';
import { Result, Button } from 'antd';
import { reportFrontendError } from '../utils/telemetry';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

/**
 * 错误边界组件
 * 捕获子组件渲染错误，防止整个应用崩溃
 */
class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('错误边界捕获异常：', error, errorInfo);
    void reportFrontendError(error, {
      source: 'ErrorBoundary',
      componentStack: errorInfo?.componentStack || null,
    });
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div style={{ padding: '48px' }}>
          <Result
            status="error"
            title="页面渲染错误"
            subTitle={
              <>
                <p>抱歉，页面遇到了渲染错误。请尝试刷新页面。</p>
                {this.state.error && (
                  <div style={{ marginTop: 16, textAlign: 'left' }}>
                    <details>
                      <summary style={{ cursor: 'pointer', color: '#ff4d4f' }}>
                        查看错误详情
                      </summary>
                      <pre
                        style={{
                          marginTop: 8,
                          padding: 12,
                          background: '#f5f5f5',
                          borderRadius: 4,
                          fontSize: 12,
                          overflow: 'auto',
                        }}
                      >
                        {this.state.error.toString()}
                        {'\n\n'}
                        {this.state.error.stack}
                      </pre>
                    </details>
                  </div>
                )}
              </>
            }
            extra={[
              <Button type="primary" key="reload" onClick={this.handleReset}>
                刷新页面
              </Button>,
              <Button key="back" onClick={() => window.history.back()}>
                返回上一页
              </Button>,
            ]}
          />
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
