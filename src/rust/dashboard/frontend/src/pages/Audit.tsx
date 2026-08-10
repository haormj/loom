import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Audit() {
  const { data } = useQuery({
    queryKey: ['auditRecords'],
    queryFn: api.auditRecords,
  });

  const sizeRecords = (data as any)?.requestSizeRecords ?? [];
  const fieldRecords = (data as any)?.fieldReadRecords ?? [];

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold">审计</h2>
      <div>
        <h3 className="mb-2 text-sm font-medium text-gray-700">请求大小审计 (最近 {sizeRecords.length} 条)</h3>
        {sizeRecords.length === 0 ? (
          <p className="text-gray-400">暂无记录</p>
        ) : (
          <pre className="max-h-64 overflow-auto rounded bg-gray-50 p-3 text-xs">
            {JSON.stringify(sizeRecords, null, 2)}
          </pre>
        )}
      </div>
      <div>
        <h3 className="mb-2 text-sm font-medium text-gray-700">字段读取审计 (最近 {fieldRecords.length} 条)</h3>
        {fieldRecords.length === 0 ? (
          <p className="text-gray-400">暂无记录</p>
        ) : (
          <pre className="max-h-64 overflow-auto rounded bg-gray-50 p-3 text-xs">
            {JSON.stringify(fieldRecords, null, 2)}
          </pre>
        )}
      </div>
    </div>
  );
}
