import { useState } from "react";
import { NavLink, Route, Routes } from "react-router-dom";
import {
  Activity,
  ScanLine,
  ClipboardCheck,
  FlaskConical,
  LayoutDashboard,
  KeyRound,
  Menu,
  X,
} from "lucide-react";
import { setOperatorToken } from "./services/api";
import Overview from "./pages/Overview";
import Inspections from "./pages/Inspections";
import Detail from "./pages/Detail";
import Models from "./pages/Models";
import Monitoring from "./pages/Monitoring";
const links = [
  ["/", "Overview", LayoutDashboard],
  ["/inspections", "Inspections", ScanLine],
  ["/reviews", "Review Queue", ClipboardCheck],
  ["/models", "Model Lab", FlaskConical],
  ["/monitoring", "MLOps & Monitoring", Activity],
] as const;
export default function App() {
  const [open, setOpen] = useState(false),
    [showToken, setShowToken] = useState(false),
    [token, setToken] = useState("");
  return (
    <div className="app-shell">
      <aside className={open ? "sidebar open" : "sidebar"}>
        <LinkBrand />
        <p className="nav-label">WORKSPACE</p>
        <nav>
          {links.map(([path, label, Icon]) => (
            <NavLink
              key={path}
              end={path === "/"}
              to={path}
              onClick={() => setOpen(false)}
            >
              <Icon size={18} />
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-foot">
          <span className="status-dot" /> Single operator · V1
          <p>
            Known defect patterns.
            <br />
            Human judgment stays in the loop.
          </p>
          <button
            className="token-button"
            onClick={() => setShowToken((v) => !v)}
          >
            <KeyRound size={15} /> Operator access
          </button>
          {showToken && (
            <div className="token-panel">
              <label htmlFor="operator-token">Local operator token</label>
              <input
                id="operator-token"
                type="password"
                value={token}
                onChange={(e) => {
                  setToken(e.target.value);
                  setOperatorToken(e.target.value);
                }}
                autoComplete="off"
              />
              <small>Kept in memory for this session.</small>
            </div>
          )}
        </div>
      </aside>
      <div className="main">
        <div className="topbar">
          <button
            className="mobile-toggle"
            aria-label="Toggle navigation"
            onClick={() => setOpen(!open)}
          >
            {open ? <X /> : <Menu />}
          </button>
          <span>Manufacturing intelligence</span>
          <span className="pill">Quality Control / 01</span>
        </div>
        <main>
          <Routes>
            <Route path="/" element={<Overview />} />
            <Route path="/inspections" element={<Inspections />} />
            <Route path="/inspections/:id" element={<Detail />} />
            <Route path="/reviews" element={<Inspections reviewQueue />} />
            <Route path="/models" element={<Models />} />
            <Route path="/monitoring" element={<Monitoring />} />
            <Route
              path="*"
              element={<p>Page not found. Choose a workspace section.</p>}
            />
          </Routes>
        </main>
        <footer>
          Intelligent Manufacturing Quality Control · Benchmark classification,
          with traceability.
        </footer>
      </div>
    </div>
  );
}
function LinkBrand() {
  return (
    <div className="brand">
      <div className="brand-mark">
        <ScanLine size={26} />
      </div>
      <strong>
        Intelligent
        <br />
        Manufacturing<span>QUALITY CONTROL</span>
      </strong>
    </div>
  );
}
