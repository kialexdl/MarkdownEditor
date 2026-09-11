export type ViewMode = "edit" | "preview" | "split";
export type DocumentKind = "markdown" | "text" | "image" | "pdf" | "unsupported";

export interface WorkspaceRoot {
  path: string;
  displayName: string;
}

export interface WorkspaceRecord {
  id: string;
  name: string;
  roots: WorkspaceRoot[];
  updatedAt: number;
}

export interface FileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  kind: DocumentKind;
  size: number;
  modifiedMs: number;
}

export interface DiskFingerprint {
  modifiedMs: number;
  size: number;
  sha256: string;
}

export interface ReadDocumentResult {
  path: string;
  name: string;
  kind: DocumentKind;
  editable: boolean;
  content: string | null;
  dataUrl: string | null;
  size: number;
  largeFile: boolean;
  hasBom: boolean;
  lineEnding: "\n" | "\r\n";
  fingerprint: DiskFingerprint;
}

export interface DocumentSession extends ReadDocumentResult {
  workspaceId: string;
  content: string;
  savedContent: string;
  viewMode: ViewMode;
  dirty: boolean;
  saving: boolean;
  conflict: boolean;
  activeLine: number;
  error?: string;
}

export interface SaveResult {
  status: "saved" | "conflict";
  fingerprint: DiskFingerprint | null;
  message: string | null;
}

export interface GitInfo {
  available: boolean;
  repositoryRoot: string | null;
  branch: string | null;
  fileStatus: string | null;
  message: string | null;
}

export interface GitHistoryEntry {
  commit: string;
  shortCommit: string;
  author: string;
  authoredAt: string;
  subject: string;
  pathAtCommit: string;
}

export interface GitHistoryPage {
  entries: GitHistoryEntry[];
  hasMore: boolean;
}

export interface RecoveryEntry {
  workspaceId: string;
  path: string;
  content: string;
  savedAt: number;
}

export interface FsEventPayload {
  workspaceId: string;
  paths: string[];
  kind: string;
}
