import { useState } from "react";
import { commands, type PingResponse, type RustacleError } from "../../../bindings";

export default function PingButton() {
  const [result, setResult] = useState<PingResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handlePing = async () => {
    setLoading(true);
    setError(null);

    const response = await commands.ping();

    if (response.status === "ok") {
      setResult(response.data);
    } else {
      const err: RustacleError = response.error;
      switch (err.kind) {
        case "NotFound":
          setError(`Not found: ${err.data.resource}`);
          break;
        case "Denied":
          setError(`Denied: ${err.data.reason}`);
          break;
        case "InvalidInput":
          setError(`Invalid: ${err.data.field}: ${err.data.message}`);
          break;
        case "Internal":
          setError(`Internal: ${err.data.message}`);
          break;
        case "PluginError":
          setError(`Plugin ${err.data.plugin_id}: ${err.data.message}`);
          break;
      }
    }

    setLoading(false);
  };

  return (
    <div className="mt-8 flex flex-col items-center gap-4">
      <button
        onClick={handlePing}
        disabled={loading}
        className="px-6 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-gray-600 text-white rounded-lg transition-colors font-medium"
      >
        {loading ? "Pinging..." : "Ping"}
      </button>

      {result && (
        <div className="text-center text-sm">
          <p className="text-green-400 font-mono">{result.message}</p>
          <p className="text-gray-500">
            {new Date(result.timestamp).toLocaleTimeString()}
          </p>
        </div>
      )}

      {error && <p className="text-red-400 text-sm">{error}</p>}
    </div>
  );
}
