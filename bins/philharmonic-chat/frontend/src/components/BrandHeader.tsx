import { type JSX, useEffect, useState } from "react";

import { type Locale } from "../i18n";
import { useT } from "../hooks/useT";
import { setAgentName, signOut } from "../store/authSlice";
import { useAppDispatch, useAppSelector } from "../store";
import { setLocale } from "../store/i18nSlice";

interface BrandHeaderProps {
  onBackToAwaiting: () => void;
}

export default function BrandHeader({ onBackToAwaiting }: BrandHeaderProps): JSX.Element {
  const t = useT();
  const dispatch = useAppDispatch();
  const { name, monogram } = useAppSelector((state) => state.branding);
  const { agentName, isSignedIn } = useAppSelector((state) => state.auth);
  const locale = useAppSelector((state) => state.i18n.locale);
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
          <span>{t.brand.productLabel}</span>
        </span>
      </button>
      {isSignedIn && (
        <div className="agent-controls">
          <label className="language-field">
            <span>{t.language.label}</span>
            <select
              value={locale}
              onChange={(event) => dispatch(setLocale(event.target.value as Locale))}
            >
              <option value="en">{t.language.english}</option>
              <option value="ja">{t.language.japanese}</option>
            </select>
          </label>
          <label className="agent-name-field">
            <span>{t.brand.agentLabel}</span>
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
            {t.common.signOut}
          </button>
        </div>
      )}
    </header>
  );
}
