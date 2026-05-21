import { type FormEvent, type JSX, useState } from "react";

import { signIn } from "../api/client";
import { useT } from "../hooks/useT";
import { setAgentToken } from "../store/authSlice";
import { useAppDispatch } from "../store";

export default function SignIn(): JSX.Element {
  const t = useT();
  const dispatch = useAppDispatch();
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  async function submit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    const value = token.trim();
    if (value.length === 0 || isSubmitting) {
      return;
    }
    setIsSubmitting(true);
    setError(null);
    try {
      await signIn(value);
      dispatch(setAgentToken(value));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : t.signIn.failureFallback);
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <main className="page sign-in-page">
      <section className="panel sign-in-panel">
        <h1>{t.signIn.title}</h1>
        <form className="stack" onSubmit={(event) => void submit(event)}>
          <label className="field">
            <span>{t.signIn.tokenLabel}</span>
            <input
              type="password"
              value={token}
              autoComplete="current-password"
              onChange={(event) => setToken(event.target.value)}
            />
          </label>
          {error !== null && <div className="alert error">{error}</div>}
          <button
            className="button primary"
            type="submit"
            disabled={isSubmitting || token.trim().length === 0}
          >
            {t.signIn.submit}
          </button>
        </form>
      </section>
    </main>
  );
}
