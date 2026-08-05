/**
 * Engine choice: `@excalidraw/excalidraw` (not tldraw).
 *
 * Why: matches the product “Excalidraw-style” freeform Board; scene JSON
 * (elements/appState/files) is mature and portable; pan/zoom/draw/text work
 * out of the box. Vue 3 has no first-party Excalidraw — we mount a small
 * React 18 island via createRoot. OpenMesh owns persistence; the engine is
 * renderer/editor only.
 */
import { createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Excalidraw } from "@excalidraw/excalidraw";
import "@excalidraw/excalidraw/index.css";
import type { BoardScene } from "./boards";

export type ExcalidrawIslandHandle = {
  unmount: () => void;
  getScene: () => BoardScene | null;
};

function stripAppState(appState: Record<string, unknown>): Record<string, unknown> {
  // Persist viewport/background; drop ephemeral UI selection noise.
  const keep = [
    "viewBackgroundColor",
    "currentItemStrokeColor",
    "currentItemBackgroundColor",
    "currentItemFillStyle",
    "currentItemStrokeWidth",
    "currentItemStrokeStyle",
    "currentItemRoughness",
    "currentItemOpacity",
    "currentItemFontFamily",
    "currentItemFontSize",
    "currentItemTextAlign",
    "currentItemStartArrowhead",
    "currentItemEndArrowhead",
    "scrollX",
    "scrollY",
    "zoom",
    "gridSize",
    "gridModeEnabled",
    "theme",
  ] as const;
  const out: Record<string, unknown> = {};
  for (const key of keep) {
    if (key in appState) out[key] = appState[key];
  }
  return out;
}

export function mountExcalidrawIsland(
  host: HTMLElement,
  opts: {
    initialScene?: BoardScene | null;
    theme?: "light" | "dark";
    onChange?: (scene: BoardScene) => void;
  },
): ExcalidrawIslandHandle {
  const root: Root = createRoot(host);
  let latest: BoardScene | null = opts.initialScene
    ? {
        elements: Array.isArray(opts.initialScene.elements)
          ? [...opts.initialScene.elements]
          : [],
        appState:
          opts.initialScene.appState && typeof opts.initialScene.appState === "object"
            ? { ...opts.initialScene.appState }
            : {},
        files:
          opts.initialScene.files && typeof opts.initialScene.files === "object"
            ? { ...opts.initialScene.files }
            : {},
      }
    : null;

  // Scene JSON is OpenMesh-owned opaque storage; cast at the engine boundary.
  const initialData = {
    elements: Array.isArray(opts.initialScene?.elements)
      ? opts.initialScene!.elements
      : [],
    appState: {
      ...(opts.initialScene?.appState ?? {}),
      theme: opts.theme ?? "light",
    },
    files: opts.initialScene?.files ?? {},
  };

  root.render(
    createElement(Excalidraw, {
      initialData: initialData as never,
      theme: opts.theme ?? "light",
      UIOptions: {
        canvasActions: {
          loadScene: false,
          saveToActiveFile: false,
          export: false,
          saveAsImage: true,
        },
      },
      onChange: (elements: readonly unknown[], appState: unknown, files: unknown) => {
        const scene: BoardScene = {
          elements: [...elements],
          appState: stripAppState(appState as Record<string, unknown>),
          files: { ...(files as Record<string, unknown>) },
        };
        latest = scene;
        opts.onChange?.(scene);
      },
    } as never),
  );

  return {
    unmount: () => root.unmount(),
    getScene: () => latest,
  };
}
