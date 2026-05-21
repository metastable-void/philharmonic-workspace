import { type JSX, useEffect, useState } from "react";

import {
  fetchBranding,
  fetchChatConfig,
  fetchWhoami,
  setRuntimeConfig,
  setRuntimeTenantId,
  type ChatConfigResponse,
} from "./api/client";
import AgentNamePrompt from "./components/AgentNamePrompt";
import BrandHeader from "./components/BrandHeader";
import VersionRefresh from "./components/VersionRefresh";
import Awaiting from "./pages/Awaiting";
import ChatTranscript from "./pages/ChatTranscript";
import SignIn from "./pages/SignIn";
import { useAppDispatch, useAppSelector } from "./store";
import { setBranding } from "./store/brandingSlice";

type View =
  | { kind: "awaiting" }
  | { kind: "agent-chat"; instanceId: string }
  | { kind: "mock-chat"; instanceId: string; token: string };

export default function App(): JSX.Element {
  const dispatch = useAppDispatch();
  const isSignedIn = useAppSelector((state) => state.auth.isSignedIn);
  const agentToken = useAppSelector((state) => state.auth.agentToken);
  const [config, setConfig] = useState<ChatConfigResponse | null>(null);
  const [configError, setConfigError] = useState<string | null>(null);
  const [view, setView] = useState<View>({ kind: "awaiting" });

  useEffect(() => {
    fetchChatConfig()
      .then((loaded) => {
        setRuntimeConfig(loaded);
        setConfig(loaded);
      })
      .catch((caught) =>
        setConfigError(caught instanceof Error ? caught.message : "config load failed"),
      );
  }, []);

  useEffect(() => {
    if (config === null) {
      return;
    }
    fetchBranding(agentToken)
      .then((branding) => dispatch(setBranding(branding)))
      .catch(() => {});
  }, [agentToken, config, dispatch]);

  // Discover the agent's tenant id once signed in so subsequent
  // API calls carry X-Tenant-Id and resolve into Tenant scope
  // instead of falling back to Operator (which the steps handler
  // rejects).
  useEffect(() => {
    if (config === null) {
      return;
    }
    if (!isSignedIn || agentToken.length === 0) {
      setRuntimeTenantId("");
      return;
    }
    fetchWhoami(agentToken)
      .then((whoami) => setRuntimeTenantId(whoami.tenant_id))
      .catch(() => {});
  }, [agentToken, config, isSignedIn]);

  if (configError !== null) {
    return (
      <main className="page">
        <div className="alert error">{configError}</div>
      </main>
    );
  }

  if (config === null) {
    return <main className="page loading">Loading...</main>;
  }

  if (!isSignedIn) {
    return (
      <>
        <VersionRefresh />
        <BrandHeader onBackToAwaiting={() => setView({ kind: "awaiting" })} />
        <SignIn />
      </>
    );
  }

  return (
    <>
      <VersionRefresh />
      <BrandHeader onBackToAwaiting={() => setView({ kind: "awaiting" })} />
      <AgentNamePrompt />
      {view.kind === "awaiting" && (
        <Awaiting
          notifyInstanceUuid={config.notify_instance_uuid}
          onOpenAgentChat={(instanceId) => setView({ kind: "agent-chat", instanceId })}
          onOpenMockChat={(instanceId, token) => setView({ kind: "mock-chat", instanceId, token })}
        />
      )}
      {view.kind === "agent-chat" && (
        <ChatTranscript
          instanceId={view.instanceId}
          mode="agent"
          token={agentToken}
          onBack={() => setView({ kind: "awaiting" })}
        />
      )}
      {view.kind === "mock-chat" && (
        <ChatTranscript
          instanceId={view.instanceId}
          mode="mock"
          token={view.token}
          onBack={() => setView({ kind: "awaiting" })}
        />
      )}
    </>
  );
}
