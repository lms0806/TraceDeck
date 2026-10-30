export type AuthKind = "password" | "privateKey" | "agent";

export interface Profile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  group: string;
  authKind?: AuthKind;
  privateKeyPath?: string;
}

export interface Tab {
  id: string;
  title: string;
  profileId?: string;
}

export interface ConnectionCheck {
  connected: boolean;
  message: string;
}
export interface RemoteEntry {
  name: string;
  path: string;
  isDir: boolean;
  details: string;
}
export interface LogSource {
  path: string;
  content: string;
}
