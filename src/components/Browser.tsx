import { ChevronLeft } from "lucide-react";

import { FileList } from "./browser/FileList";
import { LogDock } from "./browser/LogDock";
import type { Profile } from "./browser/types";
import { useRemoteDirectory } from "./browser/useRemoteDirectory";
import { useTailStream } from "./browser/useTailStream";

export function Browser({
  profile,
  active,
  onCheck,
  notify,
}: {
  profile: Profile;
  active: boolean;
  onCheck: () => void;
  notify: (text: string) => void;
}) {
  const directory = useRemoteDirectory(profile, active, notify);
  const tail = useTailStream(profile);

  return (
    <section className="file-workspace">
      <header className="server-header">
        <div>
          <span className="offline-dot" /> {profile.username}@{profile.host}:
          {profile.port}
        </div>
        <button className="connect-button" onClick={onCheck}>
          연결 확인
        </button>
      </header>
      <div className="file-toolbar">
        <button
          onClick={directory.back}
          disabled={directory.loading || !directory.canGoBack}
        >
          <ChevronLeft size={16} /> 뒤로
        </button>
        <button
          onClick={() => directory.load(directory.path, false)}
          disabled={directory.loading}
        >
          새로고침
        </button>
        <label>
          <span>원격 경로</span>
          <input
            value={directory.path}
            onChange={(event) => directory.setPath(event.target.value)}
            onKeyDown={(event) =>
              event.key === "Enter" && directory.load(directory.path, true)
            }
          />
        </label>
        <button
          className="primary"
          onClick={() => directory.load(directory.path, true)}
          disabled={directory.loading}
        >
          {directory.loading ? "불러오는 중…" : "폴더 열기"}
        </button>
      </div>
      <FileList
        entries={directory.entries}
        loaded={directory.loaded}
        loading={directory.loading}
        onOpenDirectory={(path) => directory.load(path, true)}
        onOpenFile={tail.openFile}
      />
      {tail.preview && (
        <LogDock
          preview={tail.preview}
          streamId={tail.streamId}
          onClose={tail.close}
        />
      )}
    </section>
  );
}
