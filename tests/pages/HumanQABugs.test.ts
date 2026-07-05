import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { nextTick, ref } from 'vue';
import ContextPage from '@/pages/ContextPage.vue';
import DocsPage from '@/pages/DocsPage.vue';
import NotesPage from '@/pages/NotesPage.vue';
import { searchContext, getContextHealth } from '@/lib/contextClient';
import type { DocTreeNode } from '@/lib/store';

// Mock the context client
vi.mock('@/lib/contextClient', () => ({
  refreshContext: vi.fn(),
  searchContext: vi.fn(),
  inspectContext: vi.fn(),
  getContextHealth: vi.fn(),
}));

// Shared mock route query (mutable across tests)
const mockQuery = ref<Record<string, string>>({});

vi.mock('vue-router', () => ({
  useRoute: () => ({ query: mockQuery.value, path: '/context' }),
  useRouter: () => ({ push: vi.fn() }),
}));

// Mock useStore with realistic state
const mockStore = {
  currentProject: ref({ id: 'test-project', name: 'Test Project' }),
  currentProjectPath: ref('/test/path'),
  projectDocs: ref([]),
  docsTree: ref<DocTreeNode[]>([]),
  refreshDocs: vi.fn(),
  readDoc: vi.fn(),
  projectNotes: ref<Array<{ name: string; path: string; is_dir: boolean; size: number | null; modified_at: string | null }>>([]),
  refreshNotes: vi.fn(),
  readNote: vi.fn(),
};

vi.mock('@/lib/useStore', () => ({
  useStore: () => mockStore,
}));

describe('Real Runtime Bug Regression Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQuery.value = {};
    mockStore.docsTree.value = [];
    mockStore.projectNotes.value = [];
    vi.mocked(getContextHealth).mockResolvedValue({
      path: '/test/path',
      schema_version: 1,
      sqlite_version: '3.44.0',
      journal_mode: 'wal',
      document_count: 10,
      fts_row_count: 10,
      wal_mode_effective: true,
      integrity_ok: true,
    });
  });

  // =========================================================================
  // BUG A — Focus
  // =========================================================================
  describe('BUG A — Command Palette → Search Context focus', () => {
    it('focuses input when route.query.focus === "search"', async () => {
      mockQuery.value = { focus: 'search' };

      const wrapper = mount(ContextPage, {
        global: {
          stubs: { 'lucide-vue-next': true },
        },
        attachTo: document.body,
      });

      await nextTick();
      await new Promise<void>((resolve) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            resolve();
          });
        });
      });

      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      expect(searchInput.exists()).toBe(true);
      expect(document.activeElement).toBe(searchInput.element);

      wrapper.unmount();
    });
  });

  // =========================================================================
  // BUG B — Pre-submit search state
  // =========================================================================
  describe('BUG B — Pre-submit No Results must not appear', () => {
    it('typing without Enter does not trigger search or show No Results', async () => {
      vi.mocked(searchContext).mockResolvedValue([]);

      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: {
          stubs: { 'lucide-vue-next': true },
        },
        attachTo: document.body,
      });

      await nextTick();

      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      await searchInput.setValue('deploy');
      await nextTick();

      expect(searchContext).not.toHaveBeenCalled();
      expect(wrapper.text()).not.toContain('No results for "deploy"');
      expect(wrapper.text()).toContain('Press Enter to search');

      wrapper.unmount();
    });

    it('shows No Results only after Enter with empty results', async () => {
      vi.mocked(searchContext).mockResolvedValue([]);

      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: {
          stubs: { 'lucide-vue-next': true },
        },
        attachTo: document.body,
      });

      await nextTick();

      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      await searchInput.setValue('missing-term');
      await searchInput.trigger('keyup.enter');
      await nextTick();

      expect(searchContext).toHaveBeenCalledTimes(1);
      expect(wrapper.text()).toContain('No results for "missing-term"');

      wrapper.unmount();
    });

    it('editing input after empty search does not show stale No Results for new query', async () => {
      vi.mocked(searchContext).mockResolvedValue([]);

      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: {
          stubs: { 'lucide-vue-next': true },
        },
        attachTo: document.body,
      });

      await nextTick();

      const searchInput = wrapper.find('input[placeholder="Search your context…"]');

      // Step 1: Search for "missing-term" with empty results
      await searchInput.setValue('missing-term');
      await searchInput.trigger('keyup.enter');
      await nextTick();
      expect(wrapper.text()).toContain('No results for "missing-term"');

      // Step 2: Change input to "deploy" WITHOUT pressing Enter
      await searchInput.setValue('deploy');
      await nextTick();

      expect(wrapper.text()).not.toContain('No results for "deploy"');
      expect(wrapper.text()).toContain('Press Enter to search');
      expect(searchContext).toHaveBeenCalledTimes(1);

      wrapper.unmount();
    });
  });

  // =========================================================================
  // BUG C — Nested Doc deep link
  // =========================================================================
  describe('BUG C — Nested Doc deep link opens exact source', () => {
    it('opens nested doc with parent folders expanded (one level)', async () => {
      mockStore.docsTree.value = [
        {
          name: 'architecture',
          path: 'architecture',
          nodeType: 'folder' as const,
          children: [
            {
              name: 'overview.md',
              path: 'architecture/overview.md',
              nodeType: 'file' as const,
              children: [],
              size: 200,
              modifiedAt: null,
            },
          ],
          size: null,
          modifiedAt: null,
        },
      ];

      mockQuery.value = { file: 'architecture/overview.md' };

      const wrapper = mount(DocsPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
            DocTreeItem: true,
          },
        },
        attachTo: document.body,
      });

      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));

      expect(mockStore.readDoc).toHaveBeenCalledWith('architecture/overview.md');

      const vm = wrapper.vm as any;
      expect(vm.selectedPath).toBe('architecture/overview.md');

      const expanded = vm.expandedFolders as Set<string>;
      expect(expanded.has('architecture')).toBe(true);

      wrapper.unmount();
    });

    it('opens deeply nested doc (multiple levels)', async () => {
      mockStore.docsTree.value = [
        {
          name: 'a',
          path: 'a',
          nodeType: 'folder' as const,
          children: [
            {
              name: 'b',
              path: 'a/b',
              nodeType: 'folder' as const,
              children: [
                {
                  name: 'c.md',
                  path: 'a/b/c.md',
                  nodeType: 'file' as const,
                  children: [],
                  size: 100,
                  modifiedAt: null,
                },
              ],
              size: null,
              modifiedAt: null,
            },
          ],
          size: null,
          modifiedAt: null,
        },
      ];

      mockQuery.value = { file: 'a/b/c.md' };

      const wrapper = mount(DocsPage, {
        global: {
          stubs: { 'lucide-vue-next': true, DocTreeItem: true },
        },
        attachTo: document.body,
      });

      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));

      expect(mockStore.readDoc).toHaveBeenCalledWith('a/b/c.md');

      const vm = wrapper.vm as any;
      expect(vm.selectedPath).toBe('a/b/c.md');

      const expanded = vm.expandedFolders as Set<string>;
      expect(expanded.has('a')).toBe(true);
      expect(expanded.has('a/b')).toBe(true);

      wrapper.unmount();
    });

    it('does NOT open folderA/file when folderB/file is requested', async () => {
      mockStore.docsTree.value = [
        {
          name: 'folderA',
          path: 'folderA',
          nodeType: 'folder' as const,
          children: [
            {
              name: 'readme.md',
              path: 'folderA/readme.md',
              nodeType: 'file' as const,
              children: [],
              size: 100,
              modifiedAt: null,
            },
          ],
          size: null,
          modifiedAt: null,
        },
        {
          name: 'folderB',
          path: 'folderB',
          nodeType: 'folder' as const,
          children: [
            {
              name: 'readme.md',
              path: 'folderB/readme.md',
              nodeType: 'file' as const,
              children: [],
              size: 200,
              modifiedAt: null,
            },
          ],
          size: null,
          modifiedAt: null,
        },
      ];

      mockQuery.value = { file: 'folderB/readme.md' };

      const wrapper = mount(DocsPage, {
        global: {
          stubs: { 'lucide-vue-next': true, DocTreeItem: true },
        },
        attachTo: document.body,
      });

      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));

      expect(mockStore.readDoc).toHaveBeenCalledWith('folderB/readme.md');
      expect(mockStore.readDoc).not.toHaveBeenCalledWith('folderA/readme.md');

      const vm = wrapper.vm as any;
      expect(vm.selectedPath).toBe('folderB/readme.md');

      const expanded = vm.expandedFolders as Set<string>;
      expect(expanded.has('folderB')).toBe(true);
      expect(expanded.has('folderA')).toBe(false);

      wrapper.unmount();
    });

    it('handles URL-encoded path segments gracefully', async () => {
      mockStore.docsTree.value = [
        {
          name: 'my folder',
          path: 'my folder',
          nodeType: 'folder' as const,
          children: [
            {
              name: 'doc.md',
              path: 'my folder/doc.md',
              nodeType: 'file' as const,
              children: [],
              size: 100,
              modifiedAt: null,
            },
          ],
          size: null,
          modifiedAt: null,
        },
      ];

      mockQuery.value = { file: 'my%20folder/doc.md' };

      const wrapper = mount(DocsPage, {
        global: {
          stubs: { 'lucide-vue-next': true, DocTreeItem: true },
        },
        attachTo: document.body,
      });

      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));

      expect(mockStore.readDoc).toHaveBeenCalledWith('my folder/doc.md');

      wrapper.unmount();
    });

    it('root-level doc deep-link still works (regression check)', async () => {
      mockStore.docsTree.value = [
        {
          name: 'Sample.md',
          path: 'Sample.md',
          nodeType: 'file' as const,
          children: [],
          size: 100,
          modifiedAt: null,
        },
      ];

      mockQuery.value = { file: 'Sample.md' };

      const wrapper = mount(DocsPage, {
        global: {
          stubs: { 'lucide-vue-next': true, DocTreeItem: true },
        },
        attachTo: document.body,
      });

      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));

      expect(mockStore.readDoc).toHaveBeenCalledWith('Sample.md');

      const vm = wrapper.vm as any;
      expect(vm.selectedPath).toBe('Sample.md');

      wrapper.unmount();
    });

    it('Note deep-link still works (regression check)', async () => {
      mockStore.projectNotes.value = [
        { name: 'meeting-notes.md', path: 'meeting-notes.md', is_dir: false, size: null, modified_at: null },
      ];

      mockQuery.value = { file: 'meeting-notes.md' };

      const wrapper = mount(NotesPage, {
        global: {
          stubs: { 'lucide-vue-next': true },
        },
        attachTo: document.body,
      });

      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));

      expect(mockStore.readNote).toHaveBeenCalledWith('meeting-notes.md');

      const vm = wrapper.vm as any;
      expect(vm.selectedFilename).toBe('meeting-notes.md');

      wrapper.unmount();
    });
  });
});
