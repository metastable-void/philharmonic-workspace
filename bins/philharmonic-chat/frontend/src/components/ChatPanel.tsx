import {
  type FormEvent,
  type JSX,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";

import {
  executeInstance,
  fetchLatestStep,
  parseMessages,
  type ChatMessage,
  type JsonValue,
} from "../api/client";
import { useT } from "../hooks/useT";
import { usePoll } from "../hooks/usePoll";
import { formatTimestamp } from "../util/formatTimestamp";

interface ChatPanelProps {
  instanceId: string;
  token: string;
  mode: "agent" | "mock";
  agentName: string;
}

export default function ChatPanel({
  instanceId,
  token,
  mode,
  agentName,
}: ChatPanelProps): JSX.Element {
  const t = useT();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [isInflight, setIsInflight] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const transcriptRef = useRef<HTMLDivElement | null>(null);
  const shouldAutoscroll = useRef(true);

  const refresh = useCallback(() => {
    fetchLatestStep(instanceId, token)
      .then((step) => {
        if (step !== null) {
          setMessages(parseMessages(step.output));
        }
        setError(null);
      })
      .catch((caught) => setError(messageFrom(caught, t.common.requestFailed)));
  }, [instanceId, token, t.common.requestFailed]);

  useEffect(() => {
    setMessages([]);
    setDraft("");
    setError(null);
    refresh();
  }, [refresh]);

  usePoll(true, 2000, refresh);

  useEffect(() => {
    const transcript = transcriptRef.current;
    if (transcript !== null && shouldAutoscroll.current) {
      transcript.scrollTop = transcript.scrollHeight;
    }
  }, [messages, isInflight]);

  async function sendMessage(event?: FormEvent<HTMLFormElement>): Promise<void> {
    event?.preventDefault();
    const content = draft.trim();
    if (content.length === 0 || isInflight) {
      return;
    }

    const input: JsonValue =
      mode === "agent" ? { content, agent: true, name: agentName } : { content };
    shouldAutoscroll.current = true;
    setIsInflight(true);
    setError(null);
    try {
      const response = await executeInstance(instanceId, token, input);
      setMessages(parseMessages(response.output));
      setDraft("");
    } catch (caught) {
      setError(messageFrom(caught, t.common.requestFailed));
    } finally {
      setIsInflight(false);
    }
  }

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>): void {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void sendMessage();
    }
  }

  function handleScroll(): void {
    const transcript = transcriptRef.current;
    if (transcript === null) {
      return;
    }
    const distance = transcript.scrollHeight - transcript.scrollTop - transcript.clientHeight;
    shouldAutoscroll.current = distance < 48;
  }

  return (
    <section className="panel chat-panel">
      {error !== null && <div className="alert error">{error}</div>}
      <div className="chat-transcript" ref={transcriptRef} onScroll={handleScroll}>
        {messages.map((message, index) => (
          <ChatBubble key={`${index}-${message.role}-${message.date ?? ""}`} message={message} />
        ))}
        {messages.length === 0 && <div className="empty-cell">{t.transcript.empty}</div>}
        {isInflight && <div className="chat-bubble assistant pending">{t.common.sending}</div>}
      </div>
      <form className="chat-composer" onSubmit={(event) => void sendMessage(event)}>
        <textarea
          value={draft}
          disabled={isInflight}
          placeholder={mode === "agent" ? t.transcript.composer.agent : t.transcript.composer.customer}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={handleComposerKeyDown}
        />
        <button
          className="button primary"
          type="submit"
          disabled={isInflight || draft.trim().length === 0}
        >
          {t.common.send}
        </button>
      </form>
    </section>
  );
}

interface ChatBubbleProps {
  message: ChatMessage;
}

function ChatBubble({ message }: ChatBubbleProps): JSX.Element {
  const t = useT();
  const roleClass = message.role === "user" ? "user" : "assistant";
  const label = message.name ?? (message.role === "user" ? t.transcript.role.customer : t.transcript.role.assistant);

  return (
    <article className={`chat-row ${roleClass}`}>
      <div className={`chat-bubble ${roleClass}`}>
        <span className="chat-role-label">{label}</span>
        <p>{message.content}</p>
        {typeof message.date === "number" && (
          <time className="chat-time">{formatTimestamp(message.date)}</time>
        )}
      </div>
    </article>
  );
}

function messageFrom(caught: unknown, fallback: string): string {
  return caught instanceof Error ? caught.message : fallback;
}
