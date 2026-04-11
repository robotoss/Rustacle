import { useState, useEffect } from "react";
import {
  commands,
  type PluginCallResponse,
  type PluginSummary,
  type RustacleError,
} from "../../../bindings";

function formatError(err: RustacleError): string {
  switch (err.kind) {
    case "NotFound":
      return `Not found: ${err.data.resource}`;
    case "Denied":
      return `Denied: ${err.data.reason}`;
    case "InvalidInput":
      return `Invalid: ${err.data.field}: ${err.data.message}`;
    case "Internal":
      return `Internal: ${err.data.message}`;
    case "PluginError":
      return `Plugin ${err.data.plugin_id}: ${err.data.message}`;
  }
}

export default function PluginDemo() {
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const [pingResult, setPingResult] = useState<string | null>(null);
  const [echoInput, setEchoInput] = useState("");
  const [echoResult, setEchoResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Load plugin list on mount
  useEffect(() => {
    commands.listPlugins().then((res) => {
      if (res.status === "ok") {
        setPlugins(res.data.plugins);
      }
    });
  }, []);

  const handlePluginPing = async () => {
    setLoading(true);
    setError(null);
    setPingResult(null);

    const res = await commands.pluginCall({
      plugin_id: "rustacle.demo",
      command: "ping",
      payload: "{}",
    });

    if (res.status === "ok") {
      const data = JSON.parse(res.data.data);
      setPingResult(
        `${data.message} | calls: ${data.call_count} | ${new Date(data.timestamp).toLocaleTimeString()}`
      );
    } else {
      setError(formatError(res.error));
    }

    setLoading(false);
  };

  const handleEcho = async () => {
    if (!echoInput.trim()) return;
    setError(null);
    setEchoResult(null);

    const res = await commands.pluginCall({
      plugin_id: "rustacle.demo",
      command: "echo",
      payload: JSON.stringify({ text: echoInput }),
    });

    if (res.status === "ok") {
      const data = JSON.parse(res.data.data);
      setEchoResult(`${data.echoed} (${data.length} chars)`);
    } else {
      setError(formatError(res.error));
    }
  };

  return (
    <div className="mt-6 border border-gray-700 rounded-lg p-4 max-w-md mx-auto">
      <h3 className="text-sm font-semibold text-gray-400 mb-3 uppercase tracking-wider">
        Plugin Integration
      </h3>

      {/* Loaded plugins */}
      <div className="mb-3 text-xs text-gray-500">
        Loaded plugins:{" "}
        {plugins.map((p) => (
          <span
            key={p.id}
            className="inline-block bg-green-900/40 text-green-400 px-2 py-0.5 rounded mr-1"
          >
            {p.id} v{p.version}
          </span>
        ))}
      </div>

      {/* Ping from plugin */}
      <div className="flex gap-2 mb-3">
        <button
          onClick={handlePluginPing}
          disabled={loading}
          className="px-4 py-1.5 bg-emerald-700 hover:bg-emerald-600 disabled:bg-gray-700 text-white rounded text-sm transition-colors"
        >
          {loading ? "..." : "Ping From Plugin"}
        </button>
      </div>

      {pingResult && (
        <p className="text-green-400 text-sm font-mono mb-3">{pingResult}</p>
      )}

      {/* Echo through plugin */}
      <div className="flex gap-2 mb-3">
        <input
          type="text"
          value={echoInput}
          onChange={(e) => setEchoInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleEcho()}
          placeholder="Type to echo through plugin..."
          className="flex-1 px-3 py-1.5 bg-gray-800 border border-gray-600 rounded text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-indigo-500"
        />
        <button
          onClick={handleEcho}
          className="px-4 py-1.5 bg-indigo-700 hover:bg-indigo-600 text-white rounded text-sm transition-colors"
        >
          Echo
        </button>
      </div>

      {echoResult && (
        <p className="text-blue-400 text-sm font-mono mb-2">{echoResult}</p>
      )}

      {error && <p className="text-red-400 text-sm">{error}</p>}
    </div>
  );
}
