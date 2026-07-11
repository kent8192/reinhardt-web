import React, { useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router";

const BENCHMARK_VERSION = "baseline-version";

function initialRows() {
  return Array.from({ length: 1000 }, (_, index) => ({
    id: index + 1,
    label: `Row ${index + 1}`
  }));
}

export function App() {
  const location = useLocation();
  const navigate = useNavigate();
  const [count, setCount] = useState(0);
  const [text, setText] = useState("");
  const [rows, setRows] = useState(initialRows);
  const route = location.pathname === "/detail" ? "detail" : location.pathname === "/form" ? "form" : "home";

  const routeLabel = useMemo(() => `Route: ${route}`, [route]);

  function appendRow() {
    setRows((current) => [...current, { id: current.length + 1, label: `Row ${current.length + 1}` }]);
  }

  function reorderRows() {
    setRows((current) => {
      const copy = [...current];
      copy.reverse();
      return copy;
    });
  }

  return (
    <main data-benchmark-ready="true" data-benchmark-hydrated="true" className="bench-shell">
      <h1>Vite React Benchmark</h1>
      <p data-benchmark-value="version">{BENCHMARK_VERSION}</p>

      <section data-benchmark-scenario="counter">
        <button data-benchmark-action="counter-increment" onClick={() => setCount((value) => value + 1)}>
          Increment
        </button>
        <output data-benchmark-value="counter">Counter: {count}</output>
      </section>

      <section data-benchmark-scenario="form-input">
        <label>
          Input
          <input
            data-benchmark-action="input"
            value={text}
            onChange={(event) => setText(event.target.value)}
          />
        </label>
        <output data-benchmark-value="input">{text}</output>
      </section>

      <section data-benchmark-scenario="router">
        <nav>
          <button data-benchmark-action="route-home" onClick={() => navigate("/")}>Home</button>
          <button data-benchmark-action="route-detail" onClick={() => navigate("/detail")}>Detail</button>
          <button data-benchmark-action="route-form" onClick={() => navigate("/form")}>Form</button>
        </nav>
        <output data-benchmark-value="route">{routeLabel}</output>
      </section>

      <section data-benchmark-scenario="keyed-list">
        <button data-benchmark-action="list-append" onClick={appendRow}>Append row</button>
        <button data-benchmark-action="list-reorder" onClick={reorderRows}>Reorder rows</button>
        <output data-benchmark-value="list-count">Rows: {rows.length}</output>
        <output data-benchmark-value="list-first">First: {rows[0]?.label}</output>
        <ul>
          {rows.map((row) => (
            <li key={row.id} data-benchmark-row={row.id}>{row.label}</li>
          ))}
        </ul>
      </section>
    </main>
  );
}
