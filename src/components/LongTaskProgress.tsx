import React, { useState } from 'react';
import { Modal, Progress } from 'antd';
import { useEvent } from '../api/eventBus';

interface TaskProgress {
  task_id: string;
  phase: string;
  pct: number;
  message: string;
}

export const LongTaskManager: React.FC = () => {
  const [tasks, setTasks] = useState<Map<string, TaskProgress>>(new Map());

  useEvent('long_task_progress', (payload: unknown) => {
    // Type guard to ensure payload is TaskProgress
    const taskProgress = payload as TaskProgress;
    if (taskProgress?.task_id && typeof taskProgress.pct === 'number') {
      setTasks(prev => {
        const next = new Map(prev);
        if (taskProgress.pct >= 100) {
          next.delete(taskProgress.task_id);
        } else {
          next.set(taskProgress.task_id, taskProgress);
        }
        return next;
      });
    }
  });

  return (
    <>
      {Array.from(tasks.values()).map(task => (
        <Modal
          key={task.task_id}
          open={true}
          closable={false}
          footer={null}
          title={task.phase}
        >
          <Progress percent={Math.round(task.pct)} status="active" />
          <p style={{ marginTop: 16 }}>{task.message}</p>
        </Modal>
      ))}
    </>
  );
};
