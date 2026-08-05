import { invoke } from "@tauri-apps/api/core";

export const BOARD_SCHEMA = "openmesh.board/1" as const;
export const BOARD_ENGINE = "excalidraw" as const;

/** Opaque Excalidraw scene payload (elements / appState / files). */
export type BoardScene = {
  elements?: unknown[];
  appState?: Record<string, unknown>;
  files?: Record<string, unknown>;
  [key: string]: unknown;
};

export type BoardDocument = {
  schema: string;
  id: string;
  title: string;
  engine: string;
  scene: BoardScene;
  updatedAt: number;
};

export async function listBoards(projectPath: string): Promise<BoardDocument[]> {
  return invoke("canvas_board_list", { projectPath });
}

export async function createBoard(
  projectPath: string,
  title: string,
): Promise<BoardDocument> {
  return invoke("canvas_board_create", { projectPath, title });
}

export async function loadBoard(
  projectPath: string,
  id: string,
): Promise<BoardDocument> {
  return invoke("canvas_board_load", { projectPath, id });
}

export async function upsertBoard(
  projectPath: string,
  document: unknown,
): Promise<BoardDocument> {
  return invoke("canvas_board_upsert", { projectPath, document });
}

export async function saveBoardScene(
  projectPath: string,
  id: string,
  scene: BoardScene,
): Promise<BoardDocument> {
  return invoke("canvas_board_save_scene", { projectPath, id, scene });
}

export async function deleteBoard(
  projectPath: string,
  id: string,
): Promise<void> {
  return invoke("canvas_board_delete", { projectPath, id });
}

export async function boardAddSticky(
  projectPath: string,
  text: string,
  boardId?: string,
): Promise<BoardDocument> {
  return invoke("canvas_board_add_sticky", {
    projectPath,
    text,
    boardId: boardId ?? null,
  });
}

export async function boardConnect(
  projectPath: string,
  from: string,
  to: string,
  boardId?: string,
): Promise<BoardDocument> {
  return invoke("canvas_board_connect", {
    projectPath,
    from,
    to,
    boardId: boardId ?? null,
  });
}
