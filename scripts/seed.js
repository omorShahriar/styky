(async () => {
  const inv = window.__TAURI_INTERNALS__.invoke;
  const now = Date.now(), H = 3600e3, D = 24 * H;
  const mk = (e) => inv("create_item", { edit: { title: "", bodyMd: "", dueAt: null, repeat: "none", priority: 1, tag: null, link: null, branch: null, ...e } });
  await mk({ title: "Release v2.3", bodyMd: "Cut the branch and tag.\n\n- [ ] changelog\n- [x] bump version\n- [ ] announce in **#eng**\n\nSee [runbook](https://example.com/runbook).\n", dueAt: now - 2 * H, tag: "api", priority: 2, branch: "release/2.3", link: "https://example.com/runbook" });
  await mk({ title: "Rotate API keys", dueAt: now + 30 * 60e3, tag: "infra", repeat: "monthly" });
  await mk({ title: "Review Ana's PR", dueAt: now + D + 2 * H, tag: "api" });
  await mk({ title: "Write quarterly notes", tag: "docs" });
  await mk({ title: "Book dentist", priority: 0 });
  const done = await mk({ title: "Ship hotfix 2.2.1", tag: "api" });
  await inv("complete_item", { id: done.id });
  return (await inv("list_items")).length;
})()
