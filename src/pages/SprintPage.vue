<script setup lang="ts">
import { ref, computed } from "vue";
import { useStore } from "../lib/useStore";

const { currentProject, projectSprint, projectTasks, createMockSprint, updateTask, addRecentItem } = useStore();
const selectedTaskId = ref<string | null>(null);
const statusFilter = ref<string>("all");

const filteredTasks = computed(() => {
  if (statusFilter.value === "all") return projectTasks.value;
  return projectTasks.value.filter((t) => t.status === statusFilter.value);
});

const selectedTask = computed(() =>
  projectTasks.value.find((t) => t.id === selectedTaskId.value)
);

const completedCount = computed(() => projectTasks.value.filter((t) => t.status === "completed").length);
const progress = computed(() => projectTasks.value.length ? Math.round((completedCount.value / projectTasks.value.length) * 100) : 0);

function handleCreateMockSprint() {
  if (currentProject.value) {
    createMockSprint(currentProject.value.id);
  }
}

function selectTask(taskId: string) {
  selectedTaskId.value = selectedTaskId.value === taskId ? null : taskId;
  const task = projectTasks.value.find((t) => t.id === taskId);
  if (task) {
    addRecentItem({ type: "task", title: task.title, projectId: task.projectId, sourceId: task.id });
  }
}

function statusColor(status: string): string {
  const map: Record<string, string> = { "pending": "#6b7280", "in-progress": "#3b82f6", "blocked": "#ef4444", "completed": "#22c55e" };
  return map[status] ?? "#6b7280";
}

function priorityColor(p: string): string {
  const map: Record<string, string> = { "P0": "#ef4444", "P1": "#f59e0b", "P2": "#3b82f6", "P3": "#6b7280" };
  return map[p] ?? "#6b7280";
}
</script>

<template>
  <div class="space-y-6">
    <div>
      <h1 class="text-2xl font-bold">Sprint</h1>
      <p class="text-sm mt-1" style="color: var(--muted-foreground)">Current sprint and tasks.</p>
    </div>

    <div v-if="!currentProject" class="rounded-lg border p-8 text-center" style="border-color: var(--border)">
      <p class="text-lg font-medium">No project selected</p>
      <p class="text-sm mt-2" style="color: var(--muted-foreground)">Add a project to see sprint data.</p>
    </div>

    <div v-else-if="!projectSprint" class="rounded-lg border p-8 text-center space-y-4" style="border-color: var(--border)">
      <p class="text-lg font-medium">No sprint source configured</p>
      <p class="text-sm" style="color: var(--muted-foreground)">Create a mock sprint to get started.</p>
      <button
        @click="handleCreateMockSprint"
        class="rounded-md px-4 py-2 text-sm font-medium"
        style="background: var(--foreground); color: var(--background)"
      >
        Use Mock Sprint
      </button>
    </div>

    <div v-else class="space-y-4">
      <!-- Sprint header -->
      <div class="rounded-lg border p-4" style="border-color: var(--border)">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="font-semibold">{{ projectSprint.name }}</h2>
            <span
              class="text-[10px] px-1.5 py-0.5 rounded-full font-medium mt-1 inline-block"
              :style="{ background: '#22c55e20', color: '#22c55e' }"
            >
              {{ projectSprint.status }}
            </span>
          </div>
          <div class="text-right text-sm" style="color: var(--muted-foreground)">
            <p>{{ completedCount }} of {{ projectTasks.length }} completed</p>
            <p class="text-xs">{{ progress }}%</p>
          </div>
        </div>
        <div class="mt-3 h-2 rounded-full overflow-hidden" style="background: var(--border)">
          <div class="h-full rounded-full transition-all" style="background: #22c55e" :style="{ width: progress + '%' }"></div>
        </div>
      </div>

      <!-- Filters -->
      <div class="flex gap-2">
        <button
          v-for="f in ['all', 'pending', 'in-progress', 'blocked', 'completed']"
          :key="f"
          @click="statusFilter = f"
          class="text-xs px-2 py-1 rounded-md transition-colors"
          :style="{
            background: statusFilter === f ? 'var(--foreground)' : 'transparent',
            color: statusFilter === f ? 'var(--background)' : 'var(--muted-foreground)',
            border: '1px solid var(--border)',
          }"
        >
          {{ f === 'all' ? 'All' : f }}
        </button>
      </div>

      <!-- Task list + detail -->
      <div class="flex gap-4">
        <div class="flex-1 space-y-2">
          <button
            v-for="task in filteredTasks"
            :key="task.id"
            @click="selectTask(task.id)"
            class="w-full text-left rounded-lg border p-3 transition-colors"
            style="border-color: var(--border)"
            :class="{ 'ring-1 ring-[var(--foreground)]': selectedTaskId === task.id }"
          >
            <div class="flex items-center justify-between">
              <span class="text-sm font-medium">{{ task.title }}</span>
              <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium" :style="{ background: priorityColor(task.priority) + '20', color: priorityColor(task.priority) }">
                {{ task.priority }}
              </span>
            </div>
            <div class="flex items-center gap-2 mt-1">
              <span class="text-[10px] px-1.5 py-0.5 rounded-full" :style="{ background: statusColor(task.status) + '20', color: statusColor(task.status) }">
                {{ task.status }}
              </span>
            </div>
          </button>
        </div>

        <!-- Task detail -->
        <div v-if="selectedTask" class="w-80 rounded-lg border p-4 space-y-3 self-start" style="border-color: var(--border)">
          <h3 class="font-semibold">{{ selectedTask.title }}</h3>
          <div class="space-y-2">
            <div>
              <label class="text-xs" style="color: var(--muted-foreground)">Status</label>
              <select
                :value="selectedTask.status"
                @change="updateTask(selectedTask.id, { status: ($event.target as HTMLSelectElement).value as any })"
                class="w-full rounded-md border px-2 py-1 text-sm mt-0.5"
                style="background: var(--background); border-color: var(--border); color: var(--foreground)"
              >
                <option value="pending">Pending</option>
                <option value="in-progress">In Progress</option>
                <option value="blocked">Blocked</option>
                <option value="completed">Completed</option>
              </select>
            </div>
            <div>
              <label class="text-xs" style="color: var(--muted-foreground)">Priority</label>
              <select
                :value="selectedTask.priority"
                @change="updateTask(selectedTask.id, { priority: ($event.target as HTMLSelectElement).value as any })"
                class="w-full rounded-md border px-2 py-1 text-sm mt-0.5"
                style="background: var(--background); border-color: var(--border); color: var(--foreground)"
              >
                <option value="P0">P0</option>
                <option value="P1">P1</option>
                <option value="P2">P2</option>
                <option value="P3">P3</option>
              </select>
            </div>
          </div>
          <button
            v-if="selectedTask.status !== 'completed'"
            @click="updateTask(selectedTask.id, { status: 'in-progress' })"
            class="w-full text-xs px-2 py-1.5 rounded-md"
            style="background: var(--foreground); color: var(--background)"
          >
            Mark Active
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
