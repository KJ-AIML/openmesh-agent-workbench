import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { nextTick, ref } from 'vue';
import ContextPage from '@/pages/ContextPage.vue';
import DocsPage from '@/pages/DocsPage.vue';
import NotesPage from '@/pages/NotesPage.vue';
import { searchContext, getContextHealth } from '@/lib/contextClient';
import type { DocTreeNode } from '@/lib/store';

// NOTE: These tests use the same mocks as the production components.
// The focus test uses attachTo: document.body and nextTick() to match
// the real runtime lifecycle (nextTick-based focus, not rAF).

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
  useRoute: () => ({ get query() { return mockQuery.value; }, path: '/context' }),
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

      // The production code uses nextTick() for deterministic focus.
      await nextTick();
      await nextTick();

      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      expect(searchInput.exists()).toBe(true);
      expect(document.activeElement).toBe(searchInput.element);

      wrapper.unmount();
    });

    it('re-focuses input when _ft token changes (simulated repeated navigation)', async () => {
      mockQuery.value = { focus: 'search', _ft: '1000' };
      const wrapper = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
        attachTo: document.body,
      });
      await nextTick();
      await nextTick();
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      expect(searchInput.exists()).toBe(true);
      expect(document.activeElement).toBe(searchInput.element);

      // Simulate focus loss (Command Palette Teleport removal stole focus)
      const otherInput = document.createElement('input');
      document.body.appendChild(otherInput);
      otherInput.focus();
      expect(document.activeElement).toBe(otherInput);

      // Simulate repeated navigation with a new _ft token
      mockQuery.value = { focus: 'search', _ft: '2000' };
      await flushPromises();
      await nextTick();
      await nextTick();
      expect(document.activeElement).toBe(searchInput.element);

      document.body.removeChild(otherInput);
      wrapper.unmount();
    });

    it('focuses input on repeated navigation after visiting elsewhere', async () => {
      mockQuery.value = { focus: 'search', _ft: '1001' };
      const wrapper = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
        attachTo: document.body,
      });
      await nextTick();
      await nextTick();
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      expect(document.activeElement).toBe(searchInput.element);

      // Simulate navigating elsewhere (unmount) then back (re-mount)
      // This reproduces: user on /context, navigates to another page,
      // then uses Command Palette -> Search Context again.
      wrapper.unmount();
      mockQuery.value = {};
      await flushPromises();

      // Navigate back to Search Context with a new _ft token
      mockQuery.value = { focus: 'search', _ft: '1002' };
      const wrapper2 = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
        attachTo: document.body,
      });
      await nextTick();
      await nextTick();
      const searchInput2 = wrapper2.find('input[placeholder="Search your context…"]');
      expect(document.activeElement).toBe(searchInput2.element);

      wrapper2.unmount();
    });
  });

  // =========================================================================
  // BUG B — Pre-submit search state
  // =========================================================================
  describe('BUG B — Live search state (debounced)', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });
    afterEach(() => {
      vi.useRealTimers();
    });

    it('typing triggers debounced search after 300ms (no Enter needed)', async () => {
      vi.mocked(searchContext).mockResolvedValue([]);
      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
        attachTo: document.body,
      });
      await nextTick();
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      await searchInput.setValue('deploy');
      await nextTick();
      expect(searchContext).not.toHaveBeenCalled();
      expect(wrapper.text()).not.toContain('No results for "deploy"');
      vi.advanceTimersByTime(350);
      await flushPromises();
      await nextTick();
      expect(searchContext).toHaveBeenCalledTimes(1);
      expect(wrapper.text()).toContain('No results for "deploy"');
      wrapper.unmount();
    });

    it('Enter triggers immediate search without waiting for debounce', async () => {
      vi.mocked(searchContext).mockResolvedValue([]);
      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
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

    it('editing input after empty search triggers new debounced search', async () => {
      vi.mocked(searchContext).mockResolvedValue([]);
      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
        attachTo: document.body,
      });
      await nextTick();
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      await searchInput.setValue('missing-term');
      await searchInput.trigger('keyup.enter');
      await nextTick();
      expect(wrapper.text()).toContain('No results for "missing-term"');
      expect(searchContext).toHaveBeenCalledTimes(1);
      await searchInput.setValue('deploy');
      await nextTick();
      expect(wrapper.text()).not.toContain('No results for "missing-term"');
      expect(searchContext).toHaveBeenCalledTimes(1);
      vi.advanceTimersByTime(350);
      await flushPromises();
      await nextTick();
      expect(searchContext).toHaveBeenCalledTimes(2);
      expect(wrapper.text()).toContain('No results for "deploy"');
      wrapper.unmount();
    });

    it('clearing query resets to initial state, re-typing triggers new search', async () => {
      vi.mocked(searchContext).mockResolvedValue([]);
      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
        attachTo: document.body,
      });
      await nextTick();
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      await searchInput.setValue('missing');
      await searchInput.trigger('keyup.enter');
      await nextTick();
      expect(wrapper.text()).toContain('No results for "missing"');
      expect(searchContext).toHaveBeenCalledTimes(1);
      await searchInput.setValue('');
      await nextTick();
      expect(wrapper.text()).toContain('Enter a search query');
      await searchInput.setValue('missing');
      await nextTick();
      expect(wrapper.text()).not.toContain('No results for "missing"');
      vi.advanceTimersByTime(350);
      await flushPromises();
      await nextTick();
      expect(searchContext).toHaveBeenCalledTimes(2);
      expect(wrapper.text()).toContain('No results for "missing"');
      wrapper.unmount();
    });

    it('shows results after debounced search when previous state was empty', async () => {
      vi.mocked(searchContext).mockResolvedValueOnce([]).mockResolvedValueOnce([
        {
          document_id: 'd1', source_id: 's1', source_kind: 'doc',
          project_id: 'test-project',
          canonical_ref: 'openmesh://project/test-project/doc/readme.md',
          title: 'Deploy Guide', snippet: 'How to deploy',
          sensitivity: 'private', freshness_state: 'fresh',
          observed_at: '2026-07-05T03:00:00.000Z',
        },
      ]);
      mockQuery.value = {};
      const wrapper = mount(ContextPage, {
        global: { stubs: { 'lucide-vue-next': true } },
        attachTo: document.body,
      });
      await nextTick();
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      await searchInput.setValue('missing');
      await searchInput.trigger('keyup.enter');
      await nextTick();
      expect(wrapper.text()).toContain('No results for "missing"');
      await searchInput.setValue('deploy');
      await nextTick();
      vi.advanceTimersByTime(350);
      await flushPromises();
      await nextTick();
      expect(wrapper.text()).toContain('Deploy Guide');
      expect(searchContext).toHaveBeenCalledTimes(2);
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

    it('reused component re-handles deep-link when route.query.file changes', async () => {
      mockStore.docsTree.value = [
        { name: 'folderA', path: 'folderA', nodeType: 'folder' as const,
          children: [{ name: 'doc.md', path: 'folderA/doc.md', nodeType: 'file' as const, children: [], size: 100, modifiedAt: null }],
          size: null, modifiedAt: null },
        { name: 'folderB', path: 'folderB', nodeType: 'folder' as const,
          children: [{ name: 'readme.md', path: 'folderB/readme.md', nodeType: 'file' as const, children: [], size: 200, modifiedAt: null }],
          size: null, modifiedAt: null },
      ];
      mockQuery.value = { file: 'folderA/doc.md' };
      const wrapper = mount(DocsPage, {
        global: { stubs: { 'lucide-vue-next': true, DocTreeItem: true } },
        attachTo: document.body,
      });
      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));
      expect(mockStore.readDoc).toHaveBeenCalledWith('folderA/doc.md');
      const vm = wrapper.vm as any;
      expect(vm.selectedPath).toBe('folderA/doc.md');

      // Navigate to a different doc without unmounting (component reuse)
      mockStore.readDoc.mockClear();
      mockQuery.value = { file: 'folderB/readme.md' };
      await flushPromises();
      await new Promise((r) => setTimeout(r, 100));
      expect(mockStore.readDoc).toHaveBeenCalledWith('folderB/readme.md');
      expect(vm.selectedPath).toBe('folderB/readme.md');
      const expanded = vm.expandedFolders as Set<string>;
      expect(expanded.has('folderB')).toBe(true);
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
