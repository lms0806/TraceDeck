export type TracingRecord = {
  timestamp: string;
  level: string;
  target: string;
  message: string;
};

export function parseTracing(content: string): TracingRecord[] {
  // ANSI SGR styling is terminal metadata, not part of the tracing payload.
  // eslint-disable-next-line no-control-regex -- ANSI SGR styling from terminal output.
  const ansi = /\u001b\[[0-9;]*m/g;
  return content.split("\n").flatMap((raw) => {
    const line = raw.replace(ansi, "");
    const match = line.match(
      /^(\d{4}-\d{2}-\d{2}T\S+)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+(.+?):\s+(.*)$/,
    );
    if (!match) return [];
    return [
      {
        timestamp: match[1],
        level: match[2],
        target: match[3],
        message: match[4],
      },
    ];
  });
}
