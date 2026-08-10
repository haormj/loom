import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Logs() {
  const { data: deployData } = useQuery({
    queryKey: ['deployStatus'],
    queryFn: api.deployStatus,
  });

  const logs = deployData?.logTail ?? [];

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">部署日志</h2>
      {logs.length === 0 ? (
        <p className="text-gray-400">暂无日志</p>
      ) : (
        <pre className="max-h-[calc(100vh-200px)] overflow-auto rounded bg-gray-900 p-3 text-xs text-gray-100">
          {logs.map((line, i) => {
            let cls = 'text-gray-100';
            if (line.includes('[ERROR]')) cls = 'text-red-400';
            else if (line.includes('[WARN]')) cls = 'text-yellow-400';
            else if (line.includes('[INFO]')) cls = 'text-green-400';
            return <div key={i} className={cls}>{line}</div>;
          })}
        </pre>
      )}
    </div>
  );
}
