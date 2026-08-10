interface TaskState {
  taskId: string;
  status: string;
}

interface TaskPlanRun {
  runId: string;
  taskStates: TaskState[];
  summary: { total: number; completed: number; running: number; pending: number; failed: number };
}

export default function TaskTable({ run }: { run: TaskPlanRun | null }) {
  if (!run) return <p className="text-gray-400">暂无任务数据</p>;
  return (
    <div>
      <div className="mb-2 text-sm text-gray-600">
        总计 {run.summary.total} | 完成 {run.summary.completed} | 运行中 {run.summary.running} | 待办 {run.summary.pending}
        {run.summary.failed > 0 && <span className="text-red-600"> | 失败 {run.summary.failed}</span>}
      </div>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b text-left text-gray-500">
            <th className="py-2">Task ID</th>
            <th>状态</th>
          </tr>
        </thead>
        <tbody>
          {run.taskStates.map((task) => (
            <tr key={task.taskId} className="border-b">
              <td className="py-1.5 font-mono text-xs">{task.taskId}</td>
              <td><StatusBadge status={task.status} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

import StatusBadge from './StatusBadge';
