import { createApp } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import "./style.css";

const routes = [
  { path: "/", component: App },
  { path: "/detail", component: App },
  { path: "/form", component: App }
];

const router = createRouter({
  history: createWebHistory(),
  routes
});

createApp(App).use(router).mount("#app");
