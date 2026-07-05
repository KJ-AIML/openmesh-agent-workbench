import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick, ref } from 'vue';

// Mock the context client.
vi.mock('@/lib/contextClient', () => ({
  refreshContext: vi.fn(),
  searchContext: vi.fn(),
  inspectContext: vi.fn(),
  getContextHealth: vi.fn(),
}));

// Mock vue-router with mutable query
const mockQuery = ref<Record<string, string | undefined>>({});
vi.mock('vue-router', () => ({
  useRoute: () => ({ query: mockQuery.value }),
  useRouter: () => ({ push: vi.fn() }),
}));

// Mock useStore
const mockStore = {
  currentProject: ref({ id: 'test-project', name: 'Test Project' }),
  currentProjectPath: ref('/test/path'),
  projectDocs: ref([]),
  docsTree: ref<any[]>([]),
  refreshDocs: vi.fn(),
  readDoc: vi.fn(),
  projectNotes: ref<Array<{ name: string; path: string; is_dir: boolean; size: number | null; modified_at: string | null }>>([]),
  refreshNotes: vi.fn(),
  readNote: vi.fn(),
};

vi.mock('@/lib/useStore', () => ({
  useStore: () => mockStore,
}));

import ContextPage from '@/pages/ContextPage.vue';
import DocsPage from '@/pages/DocsPage.vue';
import NotesPage from '@/pages/NotesPage.vue';
import { searchContext, getContextHealth } from '@/lib/contextClient';

describe('Human QA Bug Regression Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQuery.value = {};
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
    // Reset store
    mockStore.docsTree.value = [];
    mockStore.projectNotes.value = [];
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('TEST 1 — Real DOM Focus', () => {
    it('focuses search input when navigating with ?focus=search', async () => {
      mockQuery.value = { focus: 'search' };

      const wrapper = mount(ContextPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
          },
        },
        attachTo: document.body,
      });

      // Wait for onMounted and setTimeout to complete
      await nextTick();
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Find the search input
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      expect(searchInput.exists()).toBe(true);

      // Check that the input element is focused in the DOM
      const inputElement = searchInput.element as HTMLInputElement;
      expect(document.activeElement).toBe(inputElement);

      wrapper.unmount();
    });

    it('does NOT force focus when navigating without ?focus=search', async () => {
      mockQuery.value = {};

      const wrapper = mount(ContextPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
          },
        },
        attachTo: document.body,
      });

      await nextTick();
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Find the search input
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      expect(searchInput.exists()).toBe(true);

      // Check that the input element is NOT focused
      const inputElement = searchInput.element as HTMLInputElement;
      expect(document.activeElement).not.toBe(inputElement);

      wrapper.unmount();
    });
  });

  describe('TEST 2 — Doc Deep Link Consumption', () => {
    it('selects exact doc when route.query.file is provided', async () => {
      // Mock docs tree with a file
      mockStore.docsTree.value = [
        {
          name: 'Sample.md',
          path: 'Sample.md',
          nodeType: 'file',
          children: [],
        },
      ];

      // Mock readDoc to return content
      const mockReadDoc = vi.fn().mockResolvedValue('# Sample Content\n\nThis is a test document.');
      mockStore.readDoc = mockReadDoc;

      // Mock route with file query
      mockQuery.value = { file: 'Sample.md' };

      const wrapper = mount(DocsPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
            DocTreeItem: true,
          },
        },
      });

      await nextTick();
      await new Promise((resolve) => setTimeout(resolve, 50));

      // Verify readDoc was called with the exact file
      expect(mockReadDoc).toHaveBeenCalledWith('Sample.md');

      // Verify the content is displayed
      expect(wrapper.text()).toContain('Sample Content');

      wrapper.unmount();
    });

    it('selects nested doc when route.query.file contains path', async () => {
      // Mock docs tree with nested file
      mockStore.docsTree.value = [
        {
          name: 'folder',
          path: 'folder',
          nodeType: 'folder',
          children: [
            {
              name: 'child.md',
              path: 'folder/child.md',
              nodeType: 'file',
              children: [],
            },
          ],
        },
      ];

      const mockReadDoc = vi.fn().mockResolvedValue('# Nested Content');
      mockStore.readDoc = mockReadDoc;

      mockQuery.value = { file: 'folder/child.md' };

      const wrapper = mount(DocsPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
            DocTreeItem: true,
          },
        },
      });

      await nextTick();
      await new Promise((resolve) => setTimeout(resolve, 50));

      expect(mockReadDoc).toHaveBeenCalledWith('folder/child.md');
      expect(wrapper.text()).toContain('Nested Content');

      wrapper.unmount();
    });
  });

  describe('TEST 3 — Note Deep Link Consumption', () => {
    it('selects exact note when route.query.file is provided', async () => {
      // Mock notes with a file
      mockStore.projectNotes.value = [
        { name: 'meeting-notes.md', path: 'meeting-notes.md', is_dir: false, size: null, modified_at: null },
      ];

      const mockReadNote = vi.fn().mockResolvedValue('# Meeting Notes\n\nImportant discussion points.');
      mockStore.readNote = mockReadNote;

      mockQuery.value = { file: 'meeting-notes.md' };

      const wrapper = mount(NotesPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
          },
        },
      });

      // Wait for async operations: onMounted -> refreshNotes -> handleSelectNote -> readNote
      await nextTick();
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Verify readNote was called by the deep-link logic
      expect(mockReadNote).toHaveBeenCalledWith('meeting-notes.md');

      // Access the component's internal state to verify the note is selected
      const vm = wrapper.vm as any;
      expect(vm.selectedFilename).toBe('meeting-notes.md');
      expect(vm.selectedContent).toContain('Meeting Notes');

      // Verify the content is rendered in the DOM (via v-html="renderedContent")
      const html = wrapper.html();
      expect(html).toContain('Meeting Notes');

      wrapper.unmount();
    });
  });

  describe('TEST 4 — Type Without Enter', () => {
    it('does NOT show "No results" before pressing Enter', async () => {
      mockQuery.value = {};

      const wrapper = mount(ContextPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
          },
        },
      });

      await nextTick();

      // Type a query without pressing Enter
      const searchInput = wrapper.find('input[placeholder="Search your context…"]');
      await searchInput.setValue('deploy');

      // Verify searchContext was NOT called
      expect(searchContext).not.toHaveBeenCalled();

      // Verify "No results" is NOT shown
      expect(wrapper.text()).not.toContain('No results for "deploy"');

      // Verify "Press Enter to search" IS shown
      expect(wrapper.text()).toContain('Press Enter to search');

      wrapper.unmount();
    });
  });

  describe('TEST 5 — Edit After Previous Search', () => {
    it('does NOT show "No results for deploy" after editing from previous empty search', async () => {
      mockQuery.value = {};

      // Mock first search to return empty
      vi.mocked(searchContext).mockResolvedValueOnce([]);

      const wrapper = mount(ContextPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
          },
        },
      });

      await nextTick();

      const searchInput = wrapper.find('input[placeholder="Search your context…"]');

      // Step 1: Search for "missing-term"
      await searchInput.setValue('missing-term');
      await searchInput.trigger('keyup.enter');
      await nextTick();

      // Verify "No results for missing-term" is shown
      expect(wrapper.text()).toContain('No results for "missing-term"');

      // Step 2: Change input to "deploy" without pressing Enter
      await searchInput.setValue('deploy');
      await nextTick();

      // Verify "No results for deploy" is NOT shown
      expect(wrapper.text()).not.toContain('No results for "deploy"');

      // Verify searchContext was NOT called again
      expect(searchContext).toHaveBeenCalledTimes(1);

      // Verify we're back to "Press Enter to search" state
      expect(wrapper.text()).toContain('Press Enter to search');

      wrapper.unmount();
    });
  });

  describe('TEST 6 — Submit After Draft', () => {
    it('executes search and shows results after pressing Enter', async () => {
      mockQuery.value = {};

      // Mock search to return results
      const mockResults = [
        {
          id: 'doc-1',
          title: 'Deployment Guide',
          snippet: 'How to deploy the application...',
          kind: 'doc',
          path: 'docs/deploy.md',
          score: 0.95,
        },
      ];
      vi.mocked(searchContext).mockResolvedValueOnce(mockResults as any);

      const wrapper = mount(ContextPage, {
        global: {
          stubs: {
            'lucide-vue-next': true,
          },
        },
      });

      await nextTick();

      const searchInput = wrapper.find('input[placeholder="Search your context…"]');

      // Type query
      await searchInput.setValue('deploy');
      await nextTick();

      // Verify "Press Enter to search" is shown
      expect(wrapper.text()).toContain('Press Enter to search');

      // Press Enter
      await searchInput.trigger('keyup.enter');
      await nextTick();

      // Verify searchContext was called exactly once with correct query
      expect(searchContext).toHaveBeenCalledTimes(1);
      expect(searchContext).toHaveBeenCalledWith(
        '/test/path',
        'deploy',
        expect.objectContaining({ limit: 25 })
      );

      // Verify results are displayed
      expect(wrapper.text()).toContain('Deployment Guide');

      // Verify "Press Enter to search" is no longer shown
      expect(wrapper.text()).not.toContain('Press Enter to search');

      wrapper.unmount();
    });
  });
});
