<script setup lang="ts">
import { ref, computed } from "vue";
import { useStore } from "../lib/useStore";
import { Plus, ListTodo, FolderOpen } from "lucide-vue-next";

const {
  currentProject,
  projectSprint,
  projectTasks,
  createMockSprint,
  updateTask,
  addRecentItem,
} = useStore();

const selectedTaskId = ref<string | null>(null);
const statusFilter = ref<string>("all");

const filteredTasks = computed(() => {
  if (statusFilter.value === "all") return projectTasks.value;
  return projectTasks.value.filter((t) => t.status === statusFilter.value);
});

const selectedTask = computed(() =>
  projectTasks.value.find((t) => t.id === selectedTaskId.value),
);

const completedCount = computed(() =>
  projectTasks.value.filter((t) => t.status === "completed").length,
);
const progress = computed(() =>
  projectTasks.value.length
    ? Math.round((completedCount.value / projectTasks.value.length) * 100)
    : 0,
);

function handleCreateMockSprint() {
  if (currentProject.value) {
    createMockSprint(currentProject.value.id);
  }
}

function selectTask(taskId: string) {
  selectedTaskId.value = selectedTaskId.value === taskId ? null : taskId;
  const task = projectTasks.value.find((t) => t.id === taskId);
  if (task) {
    addRecentItem({
      type: "task",
      title: task.title,
      projectId: task.projectId,
      sourceId: task.id,
    });
  }
}

function statusColor(status: string): string {
  const map: Record<string, string> = {
    pending: "#6b7280",
    "in-progress": "#3b82f6",
    blocked: "#ef4444",
    completed: "#22c55e",
  };
  return map[status] ?? "#6b7280";
}

function priorityColor(p: string): string {
  const map: Record<string, string> = { P0: "#ef4444", P1: "#f59e0b", P2: "#3b82f6", P3: "#6b7280" };
  return map[p] ?? "#6b7280";
}
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Sprint</h1>
      <p class="text-body text-muted mt-1">
        Current sprint and tasks.
      </p>
    </div>

    <div v-if="!currentProject" class="workbench-card p-12 text-center">
      <div
        class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl"
        style="background: var(--surface-2); border: 1px solid var(--border)"
      >
        <FolderOpen class="h-7 w-7 text-subtle" />
      </div>
      <p class="text-[15px] font-semibold">No project selected</p>
      <p class="text-sm mt-1 text-muted">
        Add a project to see sprint data.
      </p>
    </div>

    <div
      v-else-if="!projectSprint"
      class="workbench-card p-12 text-center space-y-4"
    >
      <div
        class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl"
        style="background: var(--surface-2); border: 1px solid var(--border)"
      >
        <ListTodo class="h-7 w-7 text-subtle" />
      </div>
      <p class="text-[15px] font-semibold">No sprint source configured</p>
      <p class="text-sm text-muted">
        Create a mock sprint to get started.
      </p>
      <button @click="handleCreateMockSprint" class="btn-primary flex items-center gap-2 mx-auto">
        <Plus class="h-4 w-4" />
        Use Mock Sprint
      </button>
    </div>

    <div v-else class="space-y-6">
      <!-- Sprint header -->
      <div class="workbench-card p-6">
        <div class="flex items-center justify-between mb-5">
          <div>
            <h2 class="text-heading">{{ projectSprint.name }}</h2>
            <span
              class="badge badge-success mt-2 inline-block"
              >{{ projectSprint.status }}</span
            >
          </div>
          <div class="text-right">
            <p class="text-[13px] text-muted">
              {{ completedCount }} of {{ projectTasks.length }} completed
            </p>
            <p class="text-[14px] font-semibold mt-1" style="color: var(--foreground)">{{ progress }}%</p>
          </div>
        </div>
        <div class="h-2 rounded-full overflow-hidden" style="background: var(--surface-1)">
          <div
            class="h-full rounded-full transition-all"
            style="background: linear-gradient(90deg, #22c55e 0%, #16a34a 100%)"
            :style="{ width: progress + '%' }"
          ></div>
        </div>
      </div>

      <!-- Filters -->
      <div class="flex gap-1.5">
        <button
          v-for="f in ['all', 'pending', 'in-progress', 'blocked', 'completed']"
          :key="f"
          @click="statusFilter = f"
          class="btn-ghost"
          :class="
            statusFilter === f
              ? '!bg-[var(--surface-2)] !text-[var(--foreground)]'
              : ''
          "
        >
          {{ f === "all" ? "All" : f }}
        </button>
      </div>

      <!-- Task list + detail -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div class="lg:col-span-2 space-y-2">
          <button
            v-for="task in filteredTasks"
            :key="task.id"
            @click="selectTask(task.id)"
            class="w-full text-left workbench-card-compact p-4 transition-all"
            :class="
              selectedTaskId === task.id
                ? '!border-[rgba(255,255,255,0.12)]'
                : ''
            "
          >
            <div class="flex items-center justify-between">
              <span class="text-[13px] font-medium">{{ task.title }}</span>
              <span
                class="badge"
                :style="{
                  background: priorityColor(task.priority) + '20',
                  color: priorityColor(task.priority),
                  borderColor: priorityColor(task.priority) + '30',
                }"
                >{{ task.priority }}</span
              >
            </div>
            <div class="flex items-center gap-2 mt-2">
              <span
                class="badge"
                :style="{
                  background: statusColor(task.status) + '20',
                  color: statusColor(task.status),
                  borderColor: statusColor(task.status) + '30',
                }"
                >{{ task.status }}</span
              >
            </div>
          </button>
        </div>

        <!-- Task detail -->
        <div v-if="selectedTask" class="workbench-card-compact p-5 space-y-4 self-start">
          <h3 class="text-[14px] font-semibold">{{ selectedTask.title }}</h3>
          <div class="space-y-3">
            <div>
              <label class="block text-caption font-medium mb-2 text-muted">Status</label>
              <select
                :value="selectedTask.status"
                @change="
                  updateTask(selectedTask.id, {
                    status: ($event.target as HTMLSelectElement).value as any,
                  })
                "
                class="input-luxury w-full"
              >
                <option value="pending">Pending</option>
                <option value="in-progress">In Progress</option>
                <option value="blocked">Blocked</option>
                <option value="completed">Completed</option>
              </select>
            </div>
            <div>
              <label class="block text-caption font-medium mb-2 text-muted">Priority</label>
              <select
                :value="selectedTask.priority"
                @change="
                  updateTask(selectedTask.id, {
                    priority: ($event.target as HTMLSelectElement).value as any,
                  })
                "
                class="input-luxury w-full"
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
            class="btn-primary w-full"
          >
            Mark Active
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
