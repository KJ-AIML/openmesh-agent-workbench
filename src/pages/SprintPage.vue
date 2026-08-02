<script setup lang="ts">
import { ref, computed } from "vue";
import {
  Plus,
  ListTodo,
  FolderOpen,
  LayoutGrid,
  List,
  Target,
  Trash2,
  GripVertical,
} from "lucide-vue-next";
import { useStore } from "../lib/useStore";
import type { Task } from "../types";

type BoardView = "board" | "list" | "scrum";
type TaskStatus = Task["status"];

const COLUMNS: { id: TaskStatus; label: string; hint: string; color: string }[] = [
  { id: "pending", label: "Backlog", hint: "To do", color: "#6b7280" },
  { id: "in-progress", label: "In Progress", hint: "Doing", color: "#3b82f6" },
  { id: "blocked", label: "Blocked", hint: "Impediment", color: "#ef4444" },
  { id: "completed", label: "Done", hint: "Shipped", color: "#22c55e" },
];

const {
  currentProject,
  projectSprint,
  projectTasks,
  createSprint,
  createTask,
  updateTask,
  deleteTask,
  updateSprint,
  addRecentItem,
} = useStore();

const view = ref<BoardView>("board");
const selectedTaskId = ref<string | null>(null);
const draggingId = ref<string | null>(null);
const dragOverCol = ref<TaskStatus | null>(null);
const newTitle = ref("");
const showNew = ref(false);
const creating = ref(false);
const newSprintName = ref("");
const startingSprint = ref(false);

const selectedTask = computed(() =>
  projectTasks.value.find((t) => t.id === selectedTaskId.value) ?? null,
);

const counts = computed(() => {
  const c: Record<TaskStatus, number> = {
    pending: 0,
    "in-progress": 0,
    blocked: 0,
    completed: 0,
  };
  for (const t of projectTasks.value) c[t.status] += 1;
  return c;
});

const completedCount = computed(() => counts.value.completed);
const progress = computed(() =>
  projectTasks.value.length
    ? Math.round((completedCount.value / projectTasks.value.length) * 100)
    : 0,
);

const velocityHint = computed(() => {
  const total = projectTasks.value.length;
  const done = completedCount.value;
  const active = counts.value["in-progress"];
  const blocked = counts.value.blocked;
  return { total, done, active, blocked, remaining: total - done };
});

function tasksIn(status: TaskStatus): Task[] {
  return projectTasks.value
    .filter((t) => t.status === status)
    .sort((a, b) => a.priority.localeCompare(b.priority) || a.title.localeCompare(b.title));
}

function statusColor(status: string): string {
  return COLUMNS.find((c) => c.id === status)?.color ?? "#6b7280";
}

function priorityColor(p: string): string {
  const map: Record<string, string> = {
    P0: "#ef4444",
    P1: "#f59e0b",
    P2: "#3b82f6",
    P3: "#6b7280",
  };
  return map[p] ?? "#6b7280";
}

async function handleCreateSprint() {
  if (!currentProject.value || startingSprint.value) return;
  startingSprint.value = true;
  try {
    const name =
      newSprintName.value.trim() ||
      `Sprint — ${currentProject.value.name}`;
    await createSprint(name);
    newSprintName.value = "";
    showNew.value = true; // prompt user to add their first real task
  } finally {
    startingSprint.value = false;
  }
}

async function handleAddTask(status: TaskStatus = "pending") {
  const title = newTitle.value.trim();
  if (!title || creating.value) return;
  creating.value = true;
  try {
    const task = await createTask({ title, status, priority: "P2" });
    newTitle.value = "";
    showNew.value = false;
    if (task) selectedTaskId.value = task.id;
  } finally {
    creating.value = false;
  }
}

/** Quick-add from a kanban column (defaults to that column’s status). */
async function handleColumnQuickAdd(col: TaskStatus) {
  const title = newTitle.value.trim();
  if (title) {
    await handleAddTask(col);
    return;
  }
  showNew.value = true;
  // Prefer backlog status when opening empty input from non-backlog
  if (col !== "pending") {
    /* user types then Enter uses pending unless they use list view */
  }
}

function selectTask(task: Task) {
  selectedTaskId.value = selectedTaskId.value === task.id ? null : task.id;
  addRecentItem({
    type: "task",
    title: task.title,
    projectId: task.projectId,
    sourceId: task.id,
  });
}

function onDragStart(e: DragEvent, task: Task) {
  draggingId.value = task.id;
  e.dataTransfer?.setData("text/task-id", task.id);
  e.dataTransfer!.effectAllowed = "move";
}

function onDragEnd() {
  draggingId.value = null;
  dragOverCol.value = null;
}

function onDragOver(e: DragEvent, col: TaskStatus) {
  e.preventDefault();
  dragOverCol.value = col;
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
}

function onDragLeave(col: TaskStatus) {
  if (dragOverCol.value === col) dragOverCol.value = null;
}

async function onDrop(e: DragEvent, col: TaskStatus) {
  e.preventDefault();
  const id = e.dataTransfer?.getData("text/task-id") || draggingId.value;
  dragOverCol.value = null;
  draggingId.value = null;
  if (!id) return;
  const task = projectTasks.value.find((t) => t.id === id);
  if (!task || task.status === col) return;
  await updateTask(id, { status: col });
}

async function handleDeleteSelected() {
  if (!selectedTask.value) return;
  const id = selectedTask.value.id;
  selectedTaskId.value = null;
  await deleteTask(id);
}
</script>

<template>
  <div class="sprint-page animate-fade-in">
    <div class="sprint-page__head">
      <div>
        <h1 class="text-title">Sprint & Board</h1>
        <p class="text-body text-muted mt-1">
          Kanban board, backlog, and scrum pulse — persisted to your project.
        </p>
      </div>
      <div v-if="currentProject && projectSprint" class="sprint-page__actions">
        <div class="view-toggle">
          <button
            type="button"
            class="view-toggle__btn"
            :class="{ active: view === 'board' }"
            @click="view = 'board'"
          >
            <LayoutGrid class="h-3.5 w-3.5" />
            Board
          </button>
          <button
            type="button"
            class="view-toggle__btn"
            :class="{ active: view === 'list' }"
            @click="view = 'list'"
          >
            <List class="h-3.5 w-3.5" />
            List
          </button>
          <button
            type="button"
            class="view-toggle__btn"
            :class="{ active: view === 'scrum' }"
            @click="view = 'scrum'"
          >
            <Target class="h-3.5 w-3.5" />
            Scrum
          </button>
        </div>
        <button type="button" class="btn-primary inline-flex items-center gap-1.5" @click="showNew = true">
          <Plus class="h-4 w-4" />
          Add task
        </button>
      </div>
    </div>

    <!-- Empty states -->
    <div v-if="!currentProject" class="workbench-card p-12 text-center">
      <FolderOpen class="h-7 w-7 text-subtle mx-auto mb-3" />
      <p class="text-[15px] font-semibold">No project selected</p>
      <p class="text-sm mt-1 text-muted">Add a project to manage sprints.</p>
    </div>

    <div v-else-if="!projectSprint" class="workbench-card p-12 text-center space-y-4 max-w-lg mx-auto">
      <ListTodo class="h-7 w-7 text-subtle mx-auto" />
      <p class="text-[15px] font-semibold">No active sprint</p>
      <p class="text-sm text-muted">
        Create an empty sprint, then add your own tasks. Nothing is pre-filled.
      </p>
      <input
        v-model="newSprintName"
        class="input-luxury w-full text-left"
        placeholder="Sprint name (optional)"
        @keydown.enter="handleCreateSprint"
      />
      <button
        type="button"
        class="btn-primary inline-flex items-center gap-2 mx-auto"
        :disabled="startingSprint"
        @click="handleCreateSprint"
      >
        <Plus class="h-4 w-4" />
        Create empty sprint
      </button>
    </div>

    <template v-else>
      <!-- Sprint summary strip -->
      <div class="sprint-strip workbench-card-compact">
        <div class="min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <h2 class="text-[15px] font-semibold truncate">{{ projectSprint.name }}</h2>
            <select
              class="sprint-status-select"
              :value="projectSprint.status"
              @change="
                updateSprint({
                  status: ($event.target as HTMLSelectElement).value as any,
                })
              "
            >
              <option value="planned">planned</option>
              <option value="active">active</option>
              <option value="completed">completed</option>
              <option value="archived">archived</option>
            </select>
          </div>
          <p class="text-[12px] text-muted mt-1">
            {{ completedCount }}/{{ projectTasks.length }} done · {{ progress }}%
          </p>
        </div>
        <div class="sprint-strip__meter">
          <div class="sprint-strip__bar">
            <div class="sprint-strip__fill" :style="{ width: progress + '%' }" />
          </div>
          <div class="sprint-strip__stats">
            <span v-for="col in COLUMNS" :key="col.id" class="sprint-chip" :style="{ color: col.color }">
              {{ col.label }} {{ counts[col.id] }}
            </span>
          </div>
        </div>
      </div>

      <!-- New task row (always available when board is empty) -->
      <div
        v-if="showNew || projectTasks.length === 0"
        class="workbench-card-compact p-3 flex gap-2 items-center"
      >
        <input
          v-model="newTitle"
          class="input-luxury flex-1"
          placeholder="Add your first real task…"
          autofocus
          @keydown.enter="handleAddTask('pending')"
          @keydown.esc="projectTasks.length ? (showNew = false) : null"
        />
        <button
          type="button"
          class="btn-primary"
          :disabled="creating || !newTitle.trim()"
          @click="handleAddTask('pending')"
        >
          Add
        </button>
        <button
          v-if="projectTasks.length > 0"
          type="button"
          class="btn-ghost"
          @click="showNew = false"
        >
          Cancel
        </button>
      </div>

      <!-- BOARD (Kanban) -->
      <div v-if="view === 'board'" class="kanban">
        <div
          v-for="col in COLUMNS"
          :key="col.id"
          class="kanban__col"
          :class="{ 'is-over': dragOverCol === col.id }"
          @dragover="onDragOver($event, col.id)"
          @dragleave="onDragLeave(col.id)"
          @drop="onDrop($event, col.id)"
        >
          <div class="kanban__col-head">
            <span class="kanban__dot" :style="{ background: col.color }" />
            <span class="kanban__col-title">{{ col.label }}</span>
            <span class="kanban__count">{{ counts[col.id] }}</span>
            <button
              v-if="col.id === 'pending'"
              type="button"
              class="kanban__add"
              title="Add task to backlog"
              @click="handleColumnQuickAdd('pending')"
            >
              <Plus class="h-3.5 w-3.5" />
            </button>
          </div>
          <div class="kanban__cards">
            <article
              v-for="task in tasksIn(col.id)"
              :key="task.id"
              class="kanban-card"
              :class="{
                'is-selected': selectedTaskId === task.id,
                'is-dragging': draggingId === task.id,
              }"
              draggable="true"
              @dragstart="onDragStart($event, task)"
              @dragend="onDragEnd"
              @click="selectTask(task)"
            >
              <div class="kanban-card__top">
                <GripVertical class="kanban-card__grip" />
                <span
                  class="kanban-card__prio"
                  :style="{
                    color: priorityColor(task.priority),
                    borderColor: priorityColor(task.priority) + '55',
                    background: priorityColor(task.priority) + '18',
                  }"
                >{{ task.priority }}</span>
              </div>
              <p class="kanban-card__title">{{ task.title }}</p>
              <p v-if="task.description" class="kanban-card__desc">{{ task.description }}</p>
            </article>
            <p v-if="tasksIn(col.id).length === 0" class="kanban__empty">
              {{ col.id === "pending" ? "Empty — type above to add" : "Drop here" }}
            </p>
          </div>
        </div>
      </div>

      <!-- LIST -->
      <div v-else-if="view === 'list'" class="list-view workbench-card-compact overflow-hidden">
        <table class="task-table">
          <thead>
            <tr>
              <th>Task</th>
              <th>Status</th>
              <th>Priority</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="task in projectTasks"
              :key="task.id"
              :class="{ 'is-selected': selectedTaskId === task.id }"
              @click="selectTask(task)"
            >
              <td class="task-table__title">{{ task.title }}</td>
              <td>
                <select
                  class="mini-select"
                  :value="task.status"
                  @click.stop
                  @change="
                    updateTask(task.id, {
                      status: ($event.target as HTMLSelectElement).value as any,
                    })
                  "
                >
                  <option v-for="c in COLUMNS" :key="c.id" :value="c.id">{{ c.label }}</option>
                </select>
              </td>
              <td>
                <select
                  class="mini-select"
                  :value="task.priority"
                  @click.stop
                  @change="
                    updateTask(task.id, {
                      priority: ($event.target as HTMLSelectElement).value as any,
                    })
                  "
                >
                  <option value="P0">P0</option>
                  <option value="P1">P1</option>
                  <option value="P2">P2</option>
                  <option value="P3">P3</option>
                </select>
              </td>
              <td>
                <button
                  type="button"
                  class="icon-btn"
                  title="Delete"
                  @click.stop="deleteTask(task.id)"
                >
                  <Trash2 class="h-3.5 w-3.5" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- SCRUM pulse (from your real tasks only) -->
      <div v-else class="scrum-grid">
        <div class="workbench-card p-5 space-y-3">
          <h3 class="text-[13px] font-semibold">Sprint</h3>
          <p class="text-[13px] font-medium">{{ projectSprint.name }}</p>
          <p class="text-[12px] text-muted">Status: {{ projectSprint.status }}</p>
          <div class="scrum-metrics">
            <div class="scrum-metric">
              <span class="scrum-metric__n">{{ velocityHint.done }}</span>
              <span class="scrum-metric__l">Done</span>
            </div>
            <div class="scrum-metric">
              <span class="scrum-metric__n">{{ velocityHint.active }}</span>
              <span class="scrum-metric__l">Active</span>
            </div>
            <div class="scrum-metric">
              <span class="scrum-metric__n">{{ velocityHint.blocked }}</span>
              <span class="scrum-metric__l">Blocked</span>
            </div>
            <div class="scrum-metric">
              <span class="scrum-metric__n">{{ velocityHint.remaining }}</span>
              <span class="scrum-metric__l">Remaining</span>
            </div>
          </div>
        </div>
        <div class="workbench-card p-5 space-y-3">
          <h3 class="text-[13px] font-semibold">In progress & blocked</h3>
          <ul class="scrum-list">
            <li v-for="t in tasksIn('in-progress')" :key="t.id">
              <span class="dot" style="background:#3b82f6" /> {{ t.title }}
            </li>
            <li v-for="t in tasksIn('blocked')" :key="t.id">
              <span class="dot" style="background:#ef4444" /> {{ t.title }}
            </li>
            <li v-if="!tasksIn('in-progress').length && !tasksIn('blocked').length" class="text-muted text-[12px]">
              No active or blocked tasks yet — add tasks on the Board.
            </li>
          </ul>
        </div>
        <div class="workbench-card p-5 space-y-3 lg:col-span-2">
          <h3 class="text-[13px] font-semibold">Completion</h3>
          <div class="sprint-strip__bar h-3!">
            <div class="sprint-strip__fill" :style="{ width: progress + '%' }" />
          </div>
          <p class="text-[12px] text-muted">
            {{ projectTasks.length === 0 ? "No tasks yet" : `${progress}% of ${projectTasks.length} task(s) completed` }}
          </p>
        </div>
      </div>

      <!-- Detail drawer -->
      <div v-if="selectedTask" class="workbench-card p-5 space-y-4">
        <div class="flex items-start justify-between gap-3">
          <h3 class="text-[14px] font-semibold">{{ selectedTask.title }}</h3>
          <button type="button" class="icon-btn" title="Delete task" @click="handleDeleteSelected">
            <Trash2 class="h-4 w-4" />
          </button>
        </div>
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Status</label>
            <select
              class="input-luxury w-full"
              :value="selectedTask.status"
              @change="
                updateTask(selectedTask.id, {
                  status: ($event.target as HTMLSelectElement).value as any,
                })
              "
            >
              <option v-for="c in COLUMNS" :key="c.id" :value="c.id">{{ c.label }}</option>
            </select>
          </div>
          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Priority</label>
            <select
              class="input-luxury w-full"
              :value="selectedTask.priority"
              @change="
                updateTask(selectedTask.id, {
                  priority: ($event.target as HTMLSelectElement).value as any,
                })
              "
            >
              <option value="P0">P0</option>
              <option value="P1">P1</option>
              <option value="P2">P2</option>
              <option value="P3">P3</option>
            </select>
          </div>
          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Owner</label>
            <input
              class="input-luxury w-full"
              :value="selectedTask.owner || ''"
              placeholder="Optional"
              @change="
                updateTask(selectedTask.id, {
                  owner: ($event.target as HTMLInputElement).value || undefined,
                })
              "
            />
          </div>
        </div>
        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Description</label>
          <textarea
            class="input-luxury w-full min-h-[80px]"
            :value="selectedTask.description || ''"
            placeholder="Notes / acceptance criteria"
            @change="
              updateTask(selectedTask.id, {
                description: ($event.target as HTMLTextAreaElement).value || undefined,
              })
            "
          />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.sprint-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-height: 0;
}

.sprint-page__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}

.sprint-page__actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.view-toggle {
  display: inline-flex;
  padding: 3px;
  border-radius: 10px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  gap: 2px;
}

.view-toggle__btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  padding: 0 10px;
  border-radius: 7px;
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
}

.view-toggle__btn.active {
  background: var(--surface-3);
  color: var(--foreground);
  font-weight: 600;
}

.sprint-strip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.25rem;
  padding: 1rem 1.15rem;
  flex-wrap: wrap;
}

.sprint-strip__meter {
  flex: 1;
  min-width: 200px;
  max-width: 420px;
}

.sprint-strip__bar {
  height: 8px;
  border-radius: 999px;
  background: var(--surface-2);
  overflow: hidden;
}

.sprint-strip__fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, #22c55e, #16a34a);
  transition: width 0.2s ease;
}

.sprint-strip__stats {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 0.75rem;
  margin-top: 0.5rem;
}

.sprint-chip {
  font-size: 11px;
  font-weight: 600;
  opacity: 0.9;
}

.sprint-status-select {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
}

/* Kanban */
.kanban {
  display: grid;
  grid-template-columns: repeat(4, minmax(180px, 1fr));
  gap: 0.75rem;
  min-height: 420px;
  align-items: stretch;
}

@media (max-width: 1100px) {
  .kanban {
    grid-template-columns: repeat(2, minmax(180px, 1fr));
  }
}

@media (max-width: 640px) {
  .kanban {
    grid-template-columns: 1fr;
  }
}

.kanban__col {
  display: flex;
  flex-direction: column;
  min-height: 360px;
  border-radius: 14px;
  background: var(--surface-1);
  border: 1px solid var(--border);
  overflow: hidden;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.kanban__col.is-over {
  border-color: rgba(59, 130, 246, 0.45);
  background: rgba(59, 130, 246, 0.06);
}

.kanban__col-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
}

.kanban__dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  flex-shrink: 0;
}

.kanban__col-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--foreground);
}

.kanban__count {
  margin-left: auto;
  font-size: 11px;
  font-weight: 600;
  color: var(--muted-foreground);
  background: var(--surface-2);
  border-radius: 999px;
  padding: 2px 7px;
}

.kanban__add {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
}
.kanban__add:hover {
  background: var(--surface-2);
  color: var(--foreground);
}

.kanban__cards {
  flex: 1;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
}

.kanban__empty {
  font-size: 11px;
  color: var(--muted-foreground);
  text-align: center;
  padding: 1.25rem 0.5rem;
  opacity: 0.7;
}

.kanban-card {
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--card);
  padding: 10px 10px 12px;
  cursor: grab;
  transition: border-color 0.12s ease, box-shadow 0.12s ease, opacity 0.12s ease;
  text-align: left;
}

.kanban-card:hover {
  border-color: rgba(255, 255, 255, 0.12);
}

.kanban-card.is-selected {
  border-color: rgba(59, 130, 246, 0.5);
  box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.2);
}

.kanban-card.is-dragging {
  opacity: 0.45;
}

.kanban-card__top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.kanban-card__grip {
  width: 14px;
  height: 14px;
  color: var(--muted-foreground);
  opacity: 0.5;
}

.kanban-card__prio {
  font-size: 10px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 5px;
  border: 1px solid;
}

.kanban-card__title {
  font-size: 12.5px;
  font-weight: 600;
  line-height: 1.35;
  color: var(--foreground);
}

.kanban-card__desc {
  margin-top: 6px;
  font-size: 11px;
  line-height: 1.4;
  color: var(--muted-foreground);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* List */
.task-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12.5px;
}

.task-table th {
  text-align: left;
  padding: 10px 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--muted-foreground);
  border-bottom: 1px solid var(--border);
}

.task-table td {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  vertical-align: middle;
}

.task-table tr {
  cursor: pointer;
}

.task-table tr:hover,
.task-table tr.is-selected {
  background: var(--surface-highlight);
}

.task-table__title {
  font-weight: 500;
}

.mini-select {
  font-size: 11px;
  padding: 4px 6px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
}

.icon-btn:hover {
  background: var(--surface-2);
  color: var(--foreground);
}

/* Scrum */
.scrum-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

@media (max-width: 900px) {
  .scrum-grid {
    grid-template-columns: 1fr;
  }
}

.scrum-metrics {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}

.scrum-metric {
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  padding: 10px;
  text-align: center;
}

.scrum-metric__n {
  display: block;
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.03em;
}

.scrum-metric__l {
  font-size: 10px;
  font-weight: 600;
  color: var(--muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.scrum-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
}

.scrum-list li {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
  flex-shrink: 0;
}
</style>
