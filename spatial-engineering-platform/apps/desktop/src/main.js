const title = document.querySelector("#view-title");

document.querySelectorAll("nav button").forEach((button) => {
  button.addEventListener("click", () => {
    if (title) title.textContent = button.textContent || "Workspace";
  });
});

document.querySelector("#health")?.addEventListener("click", async () => {
  const output = document.querySelector("#output");
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await invoke("backend_status");
    if (output) output.textContent = String(result);
  } catch (error) {
    if (output) output.textContent = `Backend unavailable in browser preview: ${String(error)}`;
  }
});
