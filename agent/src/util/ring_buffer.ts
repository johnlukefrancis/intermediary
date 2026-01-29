// Path: agent/src/util/ring_buffer.ts
// Description: Generic circular buffer for recent file changes per repo

/**
 * Generic ring buffer that maintains a fixed-capacity circular array.
 * Items are returned newest-first when iterating.
 */
export class RingBuffer<T> {
  private readonly items: T[];
  private head = 0;
  private count = 0;

  constructor(private readonly capacity: number) {
    if (capacity <= 0) {
      throw new Error("RingBuffer capacity must be positive");
    }
    this.items = new Array<T>(capacity);
  }

  /** Add an item to the buffer, evicting oldest if at capacity */
  push(item: T): void {
    this.items[this.head] = item;
    this.head = (this.head + 1) % this.capacity;
    if (this.count < this.capacity) {
      this.count++;
    }
  }

  /** Returns items newest-first */
  toArray(): T[] {
    const result: T[] = [];
    for (let i = 0; i < this.count; i++) {
      const index = (this.head - 1 - i + this.capacity) % this.capacity;
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      result.push(this.items[index]!);
    }
    return result;
  }

  /** Number of items currently in buffer */
  get size(): number {
    return this.count;
  }

  /** Clear all items */
  clear(): void {
    this.head = 0;
    this.count = 0;
  }
}
