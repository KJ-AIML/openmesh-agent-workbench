<script setup lang="ts">
import { onMounted, ref } from "vue";

const projects = ref<string[]>([]);

onMounted(async () => {
  try {
    const res = await fetch("/api/projects");
    if (res.ok) {
      const json = await res.json();
      if (Array.isArray(json.items)) projects.value = json.items;
    }
  } catch {
    // No backend — empty list
  }
});
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">Projects</h1>
    <div v-if="projects.length === 0" class="rounded-lg border p-6" style="border-color: var(--border)">
      <p class="text-muted-foreground">No projects found</p>
    </div>
    <div v-else class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="project in projects"
        :key="project"
        class="rounded-lg border p-4"
        style="border-color: var(--border)"
      >
        <h3 class="font-semibold">{{ project }}</h3>
      </div>
    </div>
  </div>
</template>
