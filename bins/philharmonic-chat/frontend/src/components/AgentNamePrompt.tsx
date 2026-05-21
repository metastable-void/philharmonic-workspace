import { type FormEvent, type JSX, useState } from "react";

import { useT } from "../hooks/useT";
import { setAgentName } from "../store/authSlice";
import { useAppDispatch, useAppSelector } from "../store";
import Modal from "./Modal";

export default function AgentNamePrompt(): JSX.Element | null {
  const t = useT();
  const dispatch = useAppDispatch();
  const agentName = useAppSelector((state) => state.auth.agentName);
  const isSignedIn = useAppSelector((state) => state.auth.isSignedIn);
  const [draft, setDraft] = useState(agentName);

  if (!isSignedIn || agentName.length > 0) {
    return null;
  }

  function submit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    const value = draft.trim();
    if (value.length > 0) {
      dispatch(setAgentName(value));
    }
  }

  return (
    <Modal
      ariaLabel={t.agentName.promptTitle}
      onClose={() => {}}
      closeOnBackdropClick={false}
      closeOnEscape={false}
    >
      <form className="stack" onSubmit={submit}>
        <h2>{t.agentName.promptTitle}</h2>
        <label className="field">
          <span>{t.agentName.fieldLabel}</span>
          <input
            autoFocus
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            autoComplete="name"
          />
        </label>
        <div className="actions right">
          <button className="button primary" type="submit" disabled={draft.trim().length === 0}>
            {t.common.save}
          </button>
        </div>
      </form>
    </Modal>
  );
}
