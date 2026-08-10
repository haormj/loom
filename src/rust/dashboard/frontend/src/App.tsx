import { Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Overview from './pages/Overview';
import DeliveryDetail from './pages/DeliveryDetail';
import Knowledge from './pages/Knowledge';
import Deploy from './pages/Deploy';
import Logs from './pages/Logs';
import Audit from './pages/Audit';

export default function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Overview />} />
        <Route path="/deliveries/:deliveryId" element={<DeliveryDetail />} />
        <Route path="/knowledge" element={<Knowledge />} />
        <Route path="/deploy" element={<Deploy />} />
        <Route path="/logs" element={<Logs />} />
        <Route path="/audit" element={<Audit />} />
      </Routes>
    </Layout>
  );
}
