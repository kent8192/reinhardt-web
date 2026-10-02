(() => {
  const key = Symbol.for("reinhardt.native-model-form-tracker");
  if (document[key]) return;
  const owner = new AbortController();
  document[key] = owner;
  const options = { capture: true, signal: owner.signal };
  const record = (event) => {
    const input = event.target;
    if (!(input instanceof HTMLInputElement)) return;
    const name = input.getAttribute("data-reinhardt-native-interaction-field");
    const marker = name && input.form?.elements.namedItem(name);
    if (marker instanceof HTMLInputElement) marker.value = "true";
  };
  document.addEventListener("input", record, options);
  document.addEventListener("change", record, options);
  document.addEventListener("reset", (event) => {
    const form = event.target;
    if (!(form instanceof HTMLFormElement)) return;
    // Wait for later listeners and the browser's reset default action.
    setTimeout(() => {
      if (event.defaultPrevented) return;
      for (const marker of form.elements) {
        if (marker instanceof HTMLInputElement &&
            marker.name.startsWith("__reinhardt_native_edited_")) {
          marker.value = "false";
        }
      }
    }, 0);
  }, options);
  // The WASM tracker takes ownership after installing all of its listeners.
  document.addEventListener("reinhardt:model-form-tracker-ready", () => {
    owner.abort();
    delete document[key];
  }, { once: true, signal: owner.signal });
})();
