import { type JSX, useCallback, useEffect, useState } from "react";

import { fetchVersion, type VersionResponse } from "../api/client";
import { usePoll } from "../hooks/usePoll";

const VERSION_REFRESH_INTERVAL_MS = 60_000;

export default function VersionRefresh(): JSX.Element | null {
  const [initial, setInitial] = useState<VersionResponse | null>(null);
  const [changed, setChanged] = useState(false);

  const refresh = useCallback(() => {
    fetchVersion()
      .then((version) => {
        if (initial === null) {
          setInitial(version);
          return;
        }
        if (
          version.version !== initial.version ||
          version.git_commit_sha !== initial.git_commit_sha
        ) {
          setChanged(true);
        }
      })
      .catch(() => {});
  }, [initial]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  usePoll(document.visibilityState === "visible", VERSION_REFRESH_INTERVAL_MS, refresh);

  if (!changed) {
    return null;
  }

  return (
    <div className="version-banner">
      <span>A new chat UI version is available.</span>
      <button className="button primary" type="button" onClick={() => window.location.reload()}>
        Reload
      </button>
    </div>
  );
}
