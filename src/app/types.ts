export type AuthKind = "password" | "privateKey" | "agent";

export type Profile = {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  group: string;
  authKind?: AuthKind;
  privateKeyPath?: string;
};

export type Tab = { id: string; title: string; profileId?: string };

export type ConnectionCheck = { connected: boolean; message: string };
export type RemoteEntry = {
  name: string;
  path: string;
  isDir: boolean;
  details: string;
};
export type LogSource = { path: string; content: string };
