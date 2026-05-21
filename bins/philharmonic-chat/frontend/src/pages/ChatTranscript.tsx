import { type JSX } from "react";

import ChatPanel from "../components/ChatPanel";
import { useAppSelector } from "../store";

interface ChatTranscriptProps {
  instanceId: string;
  mode: "agent" | "mock";
  token: string;
  onBack: () => void;
}

export default function ChatTranscript({
  instanceId,
  mode,
  token,
  onBack,
}: ChatTranscriptProps): JSX.Element {
  const agentName = useAppSelector((state) => state.auth.agentName);

  return (
    <main className="page transcript-page">
      <div className="page-header">
        <div>
          <h1>{mode === "agent" ? "Agent transcript" : "Mock test"}</h1>
          <p className="muted mono">{instanceId}</p>
        </div>
        <button className="button secondary" type="button" onClick={onBack}>
          Back
        </button>
      </div>
      <ChatPanel instanceId={instanceId} token={token} mode={mode} agentName={agentName} />
    </main>
  );
}
