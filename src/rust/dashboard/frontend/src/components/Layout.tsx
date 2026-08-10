import { NavLink } from 'react-router-dom';
import { Activity, BookOpen, Server, FileText, Shield } from 'lucide-react';
import { useSSE } from '../hooks/useSSE';

const navItems = [
  { to: '/', label: '总览', icon: Activity },
  { to: '/deploy', label: '部署', icon: Server },
  { to: '/knowledge', label: '知识库', icon: BookOpen },
  { to: '/logs', label: '日志', icon: FileText },
  { to: '/audit', label: '审计', icon: Shield },
];

export default function Layout({ children }: { children: React.ReactNode }) {
  const { connected, statusText } = useSSE();

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b bg-white px-6 py-3">
        <h1 className="text-lg font-semibold">Loom Dashboard</h1>
        <div className="flex items-center gap-3">
          <span className={`h-2 w-2 rounded-full ${connected ? 'bg-green-500' : 'bg-orange-500'}`} />
          <span className="text-sm text-gray-600">{statusText}</span>
        </div>
      </header>
      <div className="flex flex-1 overflow-hidden">
        <nav className="w-48 border-r bg-white py-4">
          {navItems.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                `flex items-center gap-2 px-4 py-2 text-sm ${
                  isActive ? 'bg-blue-50 text-blue-700' : 'text-gray-700 hover:bg-gray-100'
                }`
              }
            >
              <Icon size={16} />
              {label}
            </NavLink>
          ))}
        </nav>
        <main className="flex-1 overflow-auto p-6">{children}</main>
      </div>
    </div>
  );
}
