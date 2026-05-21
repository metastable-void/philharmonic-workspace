import { type JSX, useEffect, useState } from "react";

import { setAgentName, signOut } from "../store/authSlice";
import { useAppDispatch, useAppSelector } from "../store";

interface BrandHeaderProps {
  onBackToAwaiting: () => void;
}

export default function BrandHeader({ onBackToAwaiting }: BrandHeaderProps): JSX.Element {
  const dispatch = useAppDispatch();
  const { name, monogram } = useAppSelector((state) => state.branding);
  const { agentName, isSignedIn } = useAppSelector((state) => state.auth);
  const [draft, setDraft] = useState(agentName);

  useEffect(() => {
    setDraft(agentName);
  }, [agentName]);

  function saveName(): void {
    const value = draft.trim();
    if (value.length > 0) {
      dispatch(setAgentName(value));
    }
  }

  return (
    <header className="brand-header">
      <button className="brand-lockup" type="button" onClick={onBackToAwaiting}>
        <span className="brand-mark">{monogram}</span>
        <span>
          <strong>{name}</strong>
          <span>Chat</span>
        </span>
      </button>
      {isSignedIn && (
        <div className="agent-controls">
          <label className="agent-name-field">
            <span>Agent</span>
            <input
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              onBlur={saveName}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.currentTarget.blur();
                }
              }}
            />
          </label>
          <button className="button secondary" type="button" onClick={() => dispatch(signOut())}>
            Sign out
          </button>
        </div>
      )}
    </header>
  );
}
