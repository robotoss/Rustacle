import { useEffect, useState } from "react";
import { commands } from "../bindings";
import PingButton from "./components/common/PingButton";

export default function App() {
  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    commands.version().then(setAppVersion);
  }, []);

  return (
    <div className="flex items-center justify-center h-screen bg-[#1a1a2e] text-gray-200">
      <div className="text-center">
        <h1 className="text-4xl font-bold mb-2">Rustacle</h1>
        <p className="text-gray-500">
          {appVersion ? `v${appVersion}` : "..."} — Agentic Terminal
        </p>
        <PingButton />
      </div>
    </div>
  );
}
