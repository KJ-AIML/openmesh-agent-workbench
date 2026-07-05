import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import DocTreeItem from "@/components/DocTreeItem.vue";
import type { DocTreeNode } from "@/lib/store";

function makeNode(overrides: Partial<DocTreeNode> = {}): DocTreeNode {
  return {
    name: "readme",
    path: "docs/readme",
    nodeType: "file",
    children: [],
    ...overrides,
  };
}

function mountItem(node: DocTreeNode, extraProps: Record<string, unknown> = {}) {
  return mount(DocTreeItem, {
    props: {
      node,
      depth: 0,
      selectedPath: null,
      expandedFolders: new Set<string>(),
      renamingPath: null,
      renameValue: "",
      dragOverPath: null,
      ...extraProps,
    },
  });
}

describe("DocTreeItem — production behavior", () => {
  it("marks folders with data-doc-folder attribute and omits it for files", () => {
    const fileItem = mountItem(makeNode({ nodeType: "file" }));
    expect(fileItem.find("button").attributes("data-doc-folder")).toBeUndefined();

    const folderItem = mountItem(
      makeNode({ nodeType: "folder", name: "src", path: "docs/src" })
    );
    expect(folderItem.find("button").attributes("data-doc-folder")).toBe(
      "docs/src"
    );
  });

  it("renders children only when the folder is in the expanded set", () => {
    const folderNode = makeNode({
      nodeType: "folder",
      name: "src",
      path: "docs/src",
      children: [
        makeNode({ name: "a.md", path: "docs/src/a.md", nodeType: "file" }),
        makeNode({ name: "b.md", path: "docs/src/b.md", nodeType: "file" }),
      ],
    });

    const collapsed = mountItem(folderNode);
    expect(collapsed.text()).not.toContain("a.md");
    expect(collapsed.text()).not.toContain("b.md");

    const expanded = mountItem(folderNode, {
      expandedFolders: new Set(["docs/src"]),
    });
    expect(expanded.text()).toContain("a.md");
    expect(expanded.text()).toContain("b.md");
  });

  it("applies sidebar-accent background when selectedPath matches", () => {
    const node = makeNode({ path: "docs/readme" });
    const selected = mountItem(node, { selectedPath: "docs/readme" });
    const notSelected = mountItem(node, { selectedPath: "docs/other" });

    expect(selected.find("button").attributes("style")).toContain(
      "--sidebar-accent"
    );
    expect(notSelected.find("button").attributes("style")).not.toContain(
      "--sidebar-accent"
    );
  });

  it("enters rename mode: renders an input with renameValue and hides the static name", () => {
    const node = makeNode({ path: "docs/readme" });

    // Not renaming: shows name span, no input.
    const idle = mountItem(node, { renamingPath: null });
    expect(idle.find("input").exists()).toBe(false);
    expect(idle.find("button span.truncate").text()).toBe("readme");

    // Renaming: shows input with renameValue, hides name span.
    const renaming = mountItem(node, {
      renamingPath: "docs/readme",
      renameValue: "draft",
    });
    const input = renaming.find("input");
    expect(input.exists()).toBe(true);
    expect((input.element as HTMLInputElement).value).toBe("draft");
    expect(renaming.find("button span.truncate").exists()).toBe(false);
  });

  it("emits select with the node payload when clicked", async () => {
    const node = makeNode({ name: "notes", path: "docs/notes" });
    const wrapper = mountItem(node);
    await wrapper.find("button").trigger("click");

    const emitted = wrapper.emitted("select");
    expect(emitted).toBeTruthy();
    expect(emitted![0][0]).toMatchObject({ path: "docs/notes", name: "notes" });
  });
});
