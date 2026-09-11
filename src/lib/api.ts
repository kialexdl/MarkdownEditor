import { invoke } from "@tauri-apps/api/core";
import type {
  DiskFingerprint,
  FileEntry,
  GitHistoryPage,
  GitInfo,
  ReadDocumentResult,
  RecoveryEntry,
  SaveResult,
  WorkspaceRecord
} from "./types";

export const api = {
  checkGit: () => invoke<string>("check_git"),
  listWorkspaces: () => invoke<WorkspaceRecord[]>("list_workspaces"),
  pickFolder: () => invoke<string | null>("pick_folder"),
  addWorkspace: (path: string) => invoke<WorkspaceRecord>("add_workspace", { path }),
  removeWorkspace: (workspaceId: string) => invoke<void>("remove_workspace", { workspaceId }),
  listDirectory: (workspaceId: string, path: string) =>
    invoke<FileEntry[]>("list_directory", { workspaceId, path }),
  createMarkdownFile: (workspaceId: string, parentPath: string, name: string) =>
    invoke<string>("create_markdown_file", { workspaceId, parentPath, name }),
  readDocument: (workspaceId: string, path: string) =>
    invoke<ReadDocumentResult>("read_document", { workspaceId, path }),
  readAsset: (workspaceId: string, documentPath: string, assetPath: string) =>
    invoke<string>("read_asset", { workspaceId, documentPath, assetPath }),
  resolveDocumentLink: (workspaceId: string, documentPath: string, href: string) =>
    invoke<string>("resolve_document_link", { workspaceId, documentPath, href }),
  saveDocument: (
    workspaceId: string,
    path: string,
    content: string,
    expected: DiskFingerprint,
    hasBom: boolean,
    force = false
  ) =>
    invoke<SaveResult>("save_document", {
      workspaceId,
      path,
      content,
      expected,
      hasBom,
      force
    }),
  openExternalUrl: (url: string) => invoke<void>("open_external_url", { url }),
  openExternalPath: (workspaceId: string, path: string) =>
    invoke<void>("open_external_path", { workspaceId, path }),
  watchWorkspace: (workspaceId: string) => invoke<void>("watch_workspace", { workspaceId }),
  unwatchWorkspace: (workspaceId: string) => invoke<void>("unwatch_workspace", { workspaceId }),
  getGitInfo: (workspaceId: string, path: string) =>
    invoke<GitInfo>("get_git_info", { workspaceId, path }),
  listGitHistory: (workspaceId: string, path: string, offset = 0, limit = 50) =>
    invoke<GitHistoryPage>("list_git_history", { workspaceId, path, offset, limit }),
  readGitRevision: (workspaceId: string, path: string, revision: string, pathAtCommit: string) =>
    invoke<string>("read_git_revision", { workspaceId, path, revision, pathAtCommit }),
  saveRecovery: (workspaceId: string, path: string, content: string) =>
    invoke<void>("save_recovery", { workspaceId, path, content }),
  listRecovery: () => invoke<RecoveryEntry[]>("list_recovery"),
  discardRecovery: (workspaceId: string, path: string) =>
    invoke<void>("discard_recovery", { workspaceId, path })
};
