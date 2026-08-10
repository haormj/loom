import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';

export default function Knowledge() {
  const { data: sources } = useQuery({
    queryKey: ['knowledgeSources'],
    queryFn: api.knowledgeSources,
  });

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">知识库</h2>
      {!sources || sources.length === 0 ? (
        <p className="text-gray-400">暂无知识源。在 agent 中运行 <code>/loom knowledge add</code> 添加。</p>
      ) : (
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b text-left text-gray-500">
              <th className="py-2">名称</th>
              <th>Source ID</th>
              <th>文档数</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            {sources.map((s: any) => (
              <tr key={s.sourceId} className="border-b hover:bg-gray-50">
                <td className="py-2">{s.name}</td>
                <td className="font-mono text-xs text-gray-500">{s.sourceId}</td>
                <td>{s.documentCount}</td>
                <td>{s.enabled ? '✓ 已启用' : '✗ 已禁用'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
