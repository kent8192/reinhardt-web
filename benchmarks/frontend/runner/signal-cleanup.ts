export type CleanupSignal = "SIGINT" | "SIGTERM";
export type SignalCleanupHandler = () => void | Promise<void>;

type SignalTarget = {
  once(signal: CleanupSignal, listener: () => void): unknown;
};

type ExitHandler = (code: number) => void;

const cleanupSignals: CleanupSignal[] = ["SIGINT", "SIGTERM"];

export class SignalCleanupRegistry {
  private readonly cleanups = new Set<SignalCleanupHandler>();
  private handlersInstalled = false;
  private handlingSignal = false;

  constructor(
    private readonly target: SignalTarget,
    private readonly exit: ExitHandler
  ) {}

  register(cleanup: SignalCleanupHandler): () => void {
    this.cleanups.add(cleanup);
    this.installSignalHandlers();

    return () => {
      this.cleanups.delete(cleanup);
    };
  }

  private installSignalHandlers(): void {
    if (this.handlersInstalled) {
      return;
    }
    this.handlersInstalled = true;
    for (const signal of cleanupSignals) {
      this.target.once(signal, () => {
        void this.handleSignal(signal);
      });
    }
  }

  private async handleSignal(signal: CleanupSignal): Promise<void> {
    if (this.handlingSignal) {
      return;
    }
    this.handlingSignal = true;
    const cleanups = [...this.cleanups];
    this.cleanups.clear();
    try {
      await Promise.allSettled(cleanups.map(async (cleanup) => cleanup()));
    } finally {
      this.exit(signal === "SIGINT" ? 130 : 143);
    }
  }
}

const processSignalCleanup = new SignalCleanupRegistry(process, (code) => process.exit(code));

export function registerSignalCleanup(cleanup: SignalCleanupHandler): () => void {
  return processSignalCleanup.register(cleanup);
}
