export interface Profile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  group: string;
  authKind?: "password" | "privateKey" | "agent";
  privateKeyPath?: string;
}

export interface RemoteEntry {
  name: string;
  path: string;
  isDir: boolean;
  details: string;
}

export interface RemoteFile {
  path: string;
  content: string;
  truncated: boolean;
}

export interface TailChunk {
  streamId: number;
  chunk: string;
}

export interface TerminalChunk {
  terminalId: number;
  chunk: string;
}

export interface RemoteSystemStatus {
  cpuPercent: number;
  diskUsedKb: number;
  diskTotalKb: number;
  diskPercent: number;
}

export interface SystemStatusChunk {
  streamId: number;
  status: RemoteSystemStatus;
}
