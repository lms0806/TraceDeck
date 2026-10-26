export type Profile = {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  group: string;
  authKind?: "password" | "privateKey" | "agent";
  privateKeyPath?: string;
};

export type RemoteEntry = {
  name: string;
  path: string;
  isDir: boolean;
  details: string;
};

export type RemoteFile = {
  path: string;
  content: string;
  truncated: boolean;
};

export type TailChunk = { streamId: number; chunk: string };
