import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { Profile, RemoteEntry, RemoteFile, TailChunk } from "./types";

export function useTailStream(profile: Profile) {
  const [preview, setPreview] = useState<RemoteFile | null>(null);
  const [streamId, setStreamId] = useState<number | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<TailChunk>("tail-chunk", (event) => {
      if (event.payload.streamId !== streamId) return;
      setPreview((current) => {
        if (!current) return current;
        const lines = `${current.content}${event.payload.chunk}`.split("\n");
        return { ...current, content: lines.slice(-2_000).join("\n") };
      });
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, [streamId]);

  useEffect(
    () => () => {
      if (streamId !== null) void invoke("stop_tail_stream", { streamId });
    },
    [streamId],
  );

  async function openFile(entry: RemoteEntry) {
    if (streamId !== null)
      await invoke("stop_tail_stream", { streamId }).catch(() => undefined);
    setPreview({
      path: entry.path,
      content: "마지막 100줄을 불러오는 중…\n",
      truncated: false,
    });
    try {
      const initial = await invoke<RemoteFile>("read_remote_file", {
        profile,
        path: entry.path,
      });
      setPreview(initial);
      setStreamId(
        await invoke<number>("start_tail_stream", {
          profile,
          path: entry.path,
        }),
      );
    } catch (error) {
      setPreview({
        path: entry.path,
        content: `실시간 로그를 열지 못했습니다.\n${String(error)}`,
        truncated: false,
      });
      setStreamId(null);
    }
  }

  function close() {
    if (streamId !== null) void invoke("stop_tail_stream", { streamId });
    setStreamId(null);
    setPreview(null);
  }

  return { preview, streamId, openFile, close };
}
