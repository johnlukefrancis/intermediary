// Path: app/src/lib/source_control/status_read_queue.ts
// Description: FIFO gate bounding how many source-control status reads run at once across every repository

export interface StatusReadQueue {
  /** Settles when the task has run, or immediately when its owner is cancelled while still waiting */
  run(owner: object, task: () => Promise<void>): Promise<void>;
  cancel(owner: object): void;
  activeCount(): number;
}

interface Waiting {
  owner: object;
  task: () => Promise<void>;
  resolve: () => void;
}

export function createStatusReadQueue(cap: number): StatusReadQueue {
  const waiting: Waiting[] = [];
  let active = 0;

  function pump(): void {
    while (active < cap) {
      const next = waiting.shift();
      if (next === undefined) return;
      active += 1;
      const done = (): void => {
        active -= 1;
        next.resolve();
        pump();
      };
      next.task().then(done, done);
    }
  }

  return {
    run(owner, task) {
      return new Promise<void>((resolve) => {
        waiting.push({ owner, task, resolve });
        pump();
      });
    },
    cancel(owner) {
      for (let index = waiting.length - 1; index >= 0; index -= 1) {
        const entry = waiting[index];
        if (entry?.owner !== owner) continue;
        waiting.splice(index, 1);
        entry.resolve();
      }
    },
    activeCount: () => active,
  };
}
