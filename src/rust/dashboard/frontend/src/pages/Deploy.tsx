import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Deploy() {
  const { data } = useQuery({
    queryKey: ['deployStatus'],
    queryFn: api.deployStatus,
    refetchInterval: 5000,
  });

  if (!data) return <p className="text-gray-400">暂无部署数据</p>;

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">部署状态</h2>
      {!data.prepared ? (
        <div className="rounded border border-gray-200 bg-gray-50 p-4 text-gray-500">
          尚未进入部署阶段
        </div>
      ) : (
        <>
          <div className="rounded-lg border bg-white p-4">
            <h3 className="mb-2 text-sm font-medium text-gray-700">实时日志</h3>
            <pre className="max-h-96 overflow-auto rounded bg-gray-900 p-3 text-xs text-gray-100">
              {data.logTail.join('\n')}
            </pre>
            {data.logRef && (
              <p className="mt-2 text-xs text-gray-500">完整日志: <code>{data.logRef}</code></p>
            )}
          </div>
          {data.failure && (
            <div className="rounded-lg border border-red-200 bg-red-50 p-4">
              <h3 className="mb-2 text-sm font-medium text-red-800">最新失败</h3>
              <pre className="overflow-auto text-xs">{JSON.stringify(data.failure, null, 2)}</pre>
            </div>
          )}
        </>
      )}
    </div>
  );
}
