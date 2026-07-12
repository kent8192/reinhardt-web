import { execFileSync, type ChildProcess } from "node:child_process";

export function terminateProcessTree(child: ChildProcess, signal: NodeJS.Signals): void {
  if (!child.pid) {
    return;
  }
  const pids = [...listDescendantPids(child.pid), child.pid];
  if (process.env.FRONTEND_BENCHMARK_DEBUG === "1") {
    console.error(`frontend-benchmark: terminating ${signal} pids=${pids.join(",")}`);
  }
  for (const pid of pids) {
    try {
      process.kill(pid, signal);
    } catch {
      if (pid === child.pid) {
        child.kill(signal);
      }
    }
  }
}

function listDescendantPids(rootPid: number): number[] {
  let output = "";
  try {
    output = execFileSync("ps", ["-axo", "pid=,ppid="], { encoding: "utf8" });
  } catch {
    return [];
  }
  const childrenByParent = new Map<number, number[]>();
  for (const line of output.trim().split("\n")) {
    const [pidText, ppidText] = line.trim().split(/\s+/);
    const pid = Number(pidText);
    const ppid = Number(ppidText);
    if (Number.isFinite(pid) && Number.isFinite(ppid)) {
      childrenByParent.set(ppid, [...(childrenByParent.get(ppid) ?? []), pid]);
    }
  }
  const descendants: number[] = [];
  const visit = (pid: number) => {
    for (const childPid of childrenByParent.get(pid) ?? []) {
      visit(childPid);
      descendants.push(childPid);
    }
  };
  visit(rootPid);
  return descendants;
}
