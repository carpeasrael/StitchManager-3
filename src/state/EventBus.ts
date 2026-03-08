type EventHandler = (data?: unknown) => void;

class EventBusClass {
  private handlers = new Map<string, Set<EventHandler>>();

  emit(event: string, data?: unknown): void {
    const set = this.handlers.get(event);
    if (set) {
      set.forEach((handler) => handler(data));
    }
  }

  on(event: string, handler: EventHandler): () => void {
    if (!this.handlers.has(event)) {
      this.handlers.set(event, new Set());
    }
    this.handlers.get(event)!.add(handler);

    return () => {
      const set = this.handlers.get(event);
      if (set) {
        set.delete(handler);
        if (set.size === 0) {
          this.handlers.delete(event);
        }
      }
    };
  }
}

export const EventBus = new EventBusClass();
