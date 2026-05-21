import { type JSX, useCallback, useState } from "react";

import { fetchLatestStep, mintEphemeral, notifyInstances } from "../api/client";
import { usePoll } from "../hooks/usePoll";
import { addAwaiting } from "../store/notifySlice";
import { useAppDispatch, useAppSelector } from "../store";
import { formatTimestamp } from "../util/formatTimestamp";
import { playNotificationSound } from "../util/notificationSound";

interface AwaitingProps {
  notifyInstanceUuid: string;
  onOpenAgentChat: (instanceId: string) => void;
  onOpenMockChat: (instanceId: string, token: string) => void;
}

export default function Awaiting({
  notifyInstanceUuid,
  onOpenAgentChat,
  onOpenMockChat,
}: AwaitingProps): JSX.Element {
  const dispatch = useAppDispatch();
  const { awaiting, seenChatUuids } = useAppSelector((state) => state.notify);
  const agentToken = useAppSelector((state) => state.auth.agentToken);
  const [toast, setToast] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isStartingMock, setIsStartingMock] = useState(false);

  const pollNotify = useCallback(() => {
    fetchLatestStep(notifyInstanceUuid, agentToken)
      .then((step) => {
        const instances = notifyInstances(step?.output ?? null);
        for (const instanceId of instances) {
          if (!seenChatUuids.includes(instanceId)) {
            dispatch(addAwaiting(instanceId));
            setToast(`New chat ${shortId(instanceId)}`);
            playNotificationSound();
            window.setTimeout(() => setToast(null), 4000);
          }
        }
        setError(null);
      })
      .catch((caught) => setError(caught instanceof Error ? caught.message : "poll failed"));
  }, [agentToken, dispatch, notifyInstanceUuid, seenChatUuids]);

  usePoll(agentToken.length > 0, 2000, pollNotify);

  async function startMockTest(): Promise<void> {
    setIsStartingMock(true);
    setError(null);
    try {
      const minted = await mintEphemeral(agentToken);
      window.localStorage.setItem(
        `ephemeral_${minted.instance_id}_token`,
        minted.ephemeral_token,
      );
      onOpenMockChat(minted.instance_id, minted.ephemeral_token);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "mock test failed");
    } finally {
      setIsStartingMock(false);
    }
  }

  return (
    <main className="page">
      <div className="page-header">
        <div>
          <h1>Awaiting chats</h1>
          <p className="muted mono">{notifyInstanceUuid}</p>
        </div>
        <button
          className="button primary"
          type="button"
          disabled={isStartingMock}
          onClick={() => void startMockTest()}
        >
          Start mock test
        </button>
      </div>
      {toast !== null && <div className="toast">{toast}</div>}
      {error !== null && <div className="alert error">{error}</div>}
      <section className="panel">
        <table>
          <thead>
            <tr>
              <th>Instance</th>
              <th>First seen</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {awaiting.map((item) => (
              <tr key={item.instanceId}>
                <td className="mono">{item.instanceId}</td>
                <td>{formatTimestamp(item.firstSeenAt)}</td>
                <td className="table-action">
                  <button
                    className="button secondary"
                    type="button"
                    onClick={() => onOpenAgentChat(item.instanceId)}
                  >
                    Open
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}

function shortId(value: string): string {
  return value.length > 12 ? `${value.slice(0, 8)}...` : value;
}
