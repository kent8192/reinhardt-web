<script setup>
import { computed, onMounted, ref } from "vue";

const BENCHMARK_VERSION = "baseline-version";

function initialRows() {
  return Array.from({ length: 1000 }, (_, index) => ({
    id: index + 1,
    label: `Row ${index + 1}`
  }));
}

const route = useRoute();
const hydrated = ref(false);
const count = ref(0);
const text = ref("");
const rows = ref(initialRows());
const routeName = computed(() => route.path === "/detail" ? "detail" : route.path === "/form" ? "form" : "home");
const routeLabel = computed(() => `Route: ${routeName.value}`);

onMounted(() => {
  hydrated.value = true;
});

function appendRow() {
  rows.value = [...rows.value, { id: rows.value.length + 1, label: `Row ${rows.value.length + 1}` }];
}

function reorderRows() {
  rows.value = [...rows.value].reverse();
}
</script>

<template>
  <main
    data-benchmark-ready="true"
    :data-benchmark-hydrated="hydrated ? 'true' : 'false'"
    class="bench-shell"
  >
    <h1>Nuxt Benchmark</h1>
    <p data-benchmark-value="version">{{ BENCHMARK_VERSION }}</p>

    <section data-benchmark-scenario="counter">
      <button data-benchmark-action="counter-increment" @click="count += 1">Increment</button>
      <output data-benchmark-value="counter">Counter: {{ count }}</output>
    </section>

    <section data-benchmark-scenario="form-input">
      <label>
        Input
        <input data-benchmark-action="input" v-model="text" />
      </label>
      <output data-benchmark-value="input">{{ text }}</output>
    </section>

    <section data-benchmark-scenario="router">
      <nav>
        <button data-benchmark-action="route-home" @click="navigateTo('/')">Home</button>
        <button data-benchmark-action="route-detail" @click="navigateTo('/detail')">Detail</button>
        <button data-benchmark-action="route-form" @click="navigateTo('/form')">Form</button>
      </nav>
      <output data-benchmark-value="route">{{ routeLabel }}</output>
    </section>

    <section data-benchmark-scenario="keyed-list">
      <button data-benchmark-action="list-append" @click="appendRow">Append row</button>
      <button data-benchmark-action="list-reorder" @click="reorderRows">Reorder rows</button>
      <output data-benchmark-value="list-count">Rows: {{ rows.length }}</output>
      <output data-benchmark-value="list-first">First: {{ rows[0]?.label }}</output>
      <ul>
        <li v-for="row in rows.slice(0, 25)" :key="row.id" :data-benchmark-row="row.id">{{ row.label }}</li>
      </ul>
    </section>
  </main>
</template>
